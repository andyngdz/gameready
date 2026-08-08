//! Report daemons that will fight gamemode over the same settings.

use crate::improvement::{
    ApplyCx, CoreCx, CoreImprovement, Improvement, ImprovementId, Privilege, Probe, StepError,
    StepPlan, Tag, Verification,
};
use crate::journal::Change;
use crate::steps::domain::COMPETING_DAEMONS;
use crate::systemd::{unit_state, SystemdError};

/// Names the daemons that already own the settings gamemode changes.
///
/// This step never changes anything, deliberately. Each of these is something
/// the user chose to install, and two of the three are the distro's own default
/// power tooling. Disabling one to make gamemode look better would be
/// gameready deciding a system-wide policy question on the user's behalf, so it
/// reports and stops.
#[derive(Debug, Default, Clone, Copy)]
pub struct Conflicts;

impl Conflicts {
    /// The step's stable id.
    #[must_use]
    pub const fn id_const() -> ImprovementId {
        ImprovementId::from_static("core.conflicts")
    }

    /// The competing daemons that are running or set to start.
    ///
    /// A `systemctl` that cannot answer stops the search rather than being read
    /// as "nothing is there": a container has no systemd, and reporting a clean
    /// machine from a query that never ran would be a lie the user acts on.
    fn live(cx: &CoreCx<'_>) -> Result<Vec<String>, SystemdError> {
        let mut live = Vec::new();
        for daemon in COMPETING_DAEMONS {
            if unit_state(cx.runner, daemon.unit)?.is_live() {
                live.push(daemon.unit.to_owned());
            }
        }
        Ok(live)
    }
}

impl Improvement for Conflicts {
    fn id(&self) -> ImprovementId {
        Self::id_const()
    }

    fn name(&self) -> &str {
        "Check for daemons that compete with gamemode"
    }

    fn short_name(&self) -> &str {
        "Competing daemons"
    }

    fn blurb(&self) -> &str {
        "Daemons that fight gamemode"
    }

    fn rationale(&self) -> &str {
        "ananicy-cpp, tuned, and power-profiles-daemon each set process \
         priorities or the CPU governor on their own schedule. With one of them \
         running, gamemode's changes get overwritten seconds later and the \
         result looks like gamemode is broken. gameready reports which one is \
         active and leaves the choice of what to do about it to you."
    }

    fn privilege(&self) -> Privilege {
        Privilege::User
    }

    fn tags(&self) -> &[Tag] {
        &[Tag::Cpu, Tag::Scheduler]
    }
}

impl CoreImprovement for Conflicts {
    fn probe(&self, cx: &CoreCx<'_>) -> Result<Probe, StepError> {
        let live = match Self::live(cx) {
            Ok(live) => live,
            Err(source) => {
                return Ok(Probe::Unknown {
                    reason: source.to_string(),
                });
            }
        };

        match live.split_first() {
            None => Ok(Probe::AlreadyApplied {
                evidence: "no competing daemons found".to_owned(),
            }),
            Some((first, rest)) => Ok(Probe::Conflict {
                with: first.clone(),
                detail: if rest.is_empty() {
                    "it overwrites what gamemode sets while a game runs".to_owned()
                } else {
                    format!("along with {}", rest.join(", "))
                },
            }),
        }
    }

    // The three below cannot run: `probe` returns `Applicable` on no path, so
    // the executor never reaches them. They exist because the lifecycle is one
    // trait, and each fails loudly rather than quietly doing nothing, so a
    // future change to the executor surfaces here instead of silently making a
    // reporting step into a mutating one.
    fn plan(&self, _cx: &CoreCx<'_>) -> Result<StepPlan, StepError> {
        Ok(StepPlan::new(self.id(), "report only, changes nothing"))
    }

    fn apply(&self, _cx: &mut ApplyCx<'_, CoreCx<'_>>) -> Result<(), StepError> {
        Err(StepError::PreconditionLost {
            step: self.id(),
            detail: "this step only reports; it has nothing to apply".to_owned(),
        })
    }

    fn verify(&self, _cx: &CoreCx<'_>) -> Result<Verification, StepError> {
        // Empty on purpose. Verification proves a change took effect, and this
        // step makes none, so a passing check here would be inventing evidence.
        Ok(Verification::new())
    }

    fn rollback(
        &self,
        _undo: &[Change],
        _cx: &mut ApplyCx<'_, CoreCx<'_>>,
    ) -> Result<(), StepError> {
        Ok(())
    }
}

#[cfg(test)]
#[path = "conflicts_test.rs"]
mod conflicts_test;
