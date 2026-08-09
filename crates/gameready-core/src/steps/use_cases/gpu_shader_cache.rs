//! Give the shader cache room, so a big game stops recompiling what it evicted.

use std::path::PathBuf;

use crate::exec::Cmd;
use crate::improvement::{
    ApplyCx, Check, CoreCx, CoreImprovement, Improvement, ImprovementId, PlannedAction, Privilege,
    Probe, StepError, StepPlan, Tag, Verification,
};
use crate::journal::{digest, Change};
use crate::steps::constants::{
    ENVIRONMENT_D_DIR, MKDIR_BIN, NOT_SET, RMDIR_BIN, SHADER_CACHE_CONF,
};
use crate::steps::domain::DetectedGpu;
use crate::steps::use_cases::gpu_shader_cache_fragment as fragment;
use crate::steps::use_cases::gpu_shader_cache_vendor::detect_vendor;
use crate::steps::use_cases::user_home::user_home;

/// What the step reports when it finds no card it has settings for.
const NO_SUPPORTED_GPU: &str = "no NVIDIA, AMD, or Intel GPU found";

/// Raises the driver's shader cache ceiling for the invoking user.
#[derive(Debug, Clone)]
pub struct ShaderCache {
    environment_d: PathBuf,
}

impl ShaderCache {
    /// The step's stable id.
    #[must_use]
    pub const fn id_const() -> ImprovementId {
        ImprovementId::from_static("core.gpu.shader-cache")
    }

    /// Resolves the fragment directory from the invoking user's home.
    #[must_use]
    pub fn detect() -> Self {
        Self {
            environment_d: user_home().join(ENVIRONMENT_D_DIR),
        }
    }

    /// Uses an explicit directory, for tests that control the path.
    #[cfg(test)]
    #[must_use]
    pub fn with_environment_d(environment_d: PathBuf) -> Self {
        Self { environment_d }
    }

    /// The fragment this step owns.
    fn fragment_path(&self) -> PathBuf {
        self.environment_d.join(SHADER_CACHE_CONF)
    }

    /// The fragment's current body, empty when it is not there yet.
    fn current_body(&self, cx: &CoreCx<'_>) -> Result<String, StepError> {
        let path = self.fragment_path();
        if !cx.runner.path_exists(&path) {
            return Ok(String::new());
        }
        cx.runner.read_to_string(&path).map_err(StepError::Exec)
    }
}

impl Improvement for ShaderCache {
    fn id(&self) -> ImprovementId {
        Self::id_const()
    }

    fn name(&self) -> &str {
        "Give the shader cache room to keep what it compiled"
    }

    fn short_name(&self) -> &str {
        "shader cache"
    }

    fn blurb(&self) -> &str {
        "Shader cache size"
    }

    fn gains(&self) -> Option<&str> {
        Some("Less shader compilation stutter in a game you have played before. Applies from your next login.")
    }

    fn undo_note(&self) -> Option<&str> {
        Some("takes effect on your next login, like the change itself")
    }

    fn rationale(&self) -> &str {
        "Both NVIDIA's driver and Mesa cap the on-disk shader cache at 1GB by \
         default. One large title can fill that on its own, and once it is full \
         the driver evicts shaders it will need again, so they are recompiled \
         on the next launch. That recompilation is the stutter people notice in \
         the first minutes of a session. Raising the ceiling to 12GB does not \
         reserve the space; a small library never uses it."
    }

    fn privilege(&self) -> Privilege {
        Privilege::User
    }

    fn tags(&self) -> &[Tag] {
        &[Tag::Gpu]
    }
}

impl CoreImprovement for ShaderCache {
    fn probe(&self, cx: &CoreCx<'_>) -> Result<Probe, StepError> {
        let DetectedGpu::Recognised(vendor) = detect_vendor(cx.runner)? else {
            return Ok(Probe::NotApplicable {
                reason: NO_SUPPORTED_GPU.to_owned(),
            });
        };

        if fragment::sets_everything(&self.current_body(cx)?, vendor) {
            return Ok(Probe::AlreadyApplied {
                evidence: format!("{} already sets it", self.fragment_path().display()),
            });
        }
        Ok(Probe::Applicable)
    }

    fn plan(&self, cx: &CoreCx<'_>) -> Result<StepPlan, StepError> {
        let DetectedGpu::Recognised(vendor) = detect_vendor(cx.runner)? else {
            return Ok(StepPlan::new(self.id(), NO_SUPPORTED_GPU.to_owned()));
        };

        Ok(StepPlan::new(
            self.id(),
            format!("{} shader cache to 12GB", vendor.label()),
        )
        .action(PlannedAction::CreateFile {
            path: self.fragment_path().to_string_lossy().into_owned(),
            contents: fragment::preview(vendor),
        }))
    }

    fn apply(&self, cx: &mut ApplyCx<'_, CoreCx<'_>>) -> Result<(), StepError> {
        let DetectedGpu::Recognised(vendor) = detect_vendor(cx.reader())? else {
            return Err(StepError::PreconditionLost {
                step: self.id(),
                detail: NO_SUPPORTED_GPU.to_owned(),
            });
        };

        if !cx.reader().path_exists(&self.environment_d) {
            let dir = self.environment_d.clone();
            cx.mutate(
                Change::DirCreated {
                    path: dir.clone(),
                    privilege: Privilege::User,
                },
                |runner| {
                    let mkdir = Cmd::user(MKDIR_BIN)
                        .arg("-p")
                        .arg(dir.to_string_lossy().into_owned());
                    runner.run(&mkdir).map(|_| ()).map_err(StepError::Exec)
                },
            )?;
        }

        let path = self.fragment_path();
        let contents = fragment::contents(vendor, self.id(), cx.run());
        let sha256_after = digest(&contents);
        cx.mutate(
            Change::FileWritten {
                path: path.clone(),
                existed: false,
                backup: None,
                sha256_after,
                mode: 0o644,
                privilege: Privilege::User,
            },
            |runner| {
                runner
                    .write_file(&path, &contents, Privilege::User)
                    .map_err(StepError::Exec)
            },
        )
    }

    fn verify(&self, cx: &CoreCx<'_>) -> Result<Verification, StepError> {
        let path = self.fragment_path();
        let body = self.current_body(cx)?;

        let mut verification = Verification::new().check(Check::equals(
            format!("{} exists", path.display()),
            "yes",
            if body.is_empty() { "no" } else { "yes" },
        ));

        if let DetectedGpu::Recognised(vendor) = detect_vendor(cx.runner)? {
            for (key, value) in vendor.cache_settings() {
                let assignment = format!("{key}={value}");
                let actual = if body.contains(&assignment) {
                    value.clone()
                } else {
                    NOT_SET.to_owned()
                };
                verification = verification.check(Check::equals(key, value, actual));
            }
        }
        Ok(verification)
    }

    fn rollback(&self, undo: &[Change], cx: &mut ApplyCx<'_, CoreCx<'_>>) -> Result<(), StepError> {
        // Reverse order: the fragment goes before the directory that holds it.
        for change in undo.iter().rev() {
            match change {
                Change::FileWritten { path, .. } => {
                    cx.reader()
                        .remove_file(path, Privilege::User)
                        .map_err(StepError::Exec)?;
                }
                Change::DirCreated { path, .. } => {
                    // rmdir, not rm -rf, and a failure is ignored on purpose:
                    // environment.d is a shared XDG directory, so anything else
                    // that has since put a fragment there keeps both.
                    let rmdir = Cmd::user(RMDIR_BIN).arg(path.to_string_lossy().into_owned());
                    let _ = cx.reader().run_allowing_failure(&rmdir);
                }
                // Listed rather than wildcarded: a new Change variant this
                // step starts recording must fail to compile here rather than
                // be silently skipped by rollback.
                Change::FileRemoved { .. }
                | Change::SysctlRuntime { .. }
                | Change::SysfsWrite { .. }
                | Change::PackagesInstalled { .. }
                | Change::SystemdUnit { .. }
                | Change::AptRepository { .. }
                | Change::ScxScheduler { .. }
                | Change::DirTreeInstalled { .. } => {}
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "gpu_shader_cache_test.rs"]
mod gpu_shader_cache_test;
