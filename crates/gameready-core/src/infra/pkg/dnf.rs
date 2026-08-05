//! Fedora and its derivatives.

use crate::exec::{Cmd, CommandRunner};
use crate::facts::PackageManagerKind;
use crate::infra::exec::constants::INSTALL;
use crate::pkg::{InstallOutcome, PackageError, PackageManager, PackageState};

/// Fedora's package manager.
const DNF: &str = "dnf";

/// Reads the installed version without consulting any repository.
const RPM: &str = "rpm";

/// Drives `dnf`.
#[derive(Debug, Default, Clone, Copy)]
pub struct Dnf;

impl PackageManager for Dnf {
    fn kind(&self) -> PackageManagerKind {
        PackageManagerKind::Dnf
    }

    fn state(
        &self,
        runner: &dyn CommandRunner,
        package: &str,
    ) -> Result<PackageState, PackageError> {
        // rpm rather than dnf for the installed check: it reads the local
        // database and cannot be slowed down or confused by repository state.
        let installed = Cmd::user(RPM)
            .arg("-q")
            .arg("--queryformat=%{VERSION}-%{RELEASE}")
            .arg(package);
        let query =
            runner
                .run_allowing_failure(&installed)
                .map_err(|source| PackageError::Query {
                    manager: self.kind(),
                    package: package.to_owned(),
                    source,
                })?;

        if query.code == 0 && !query.stdout_trimmed().is_empty() {
            return Ok(PackageState::Installed {
                version: Some(query.stdout_trimmed().to_owned()),
            });
        }

        let available = Cmd::user(DNF).arg("info").arg("--quiet").arg(package);
        let lookup =
            runner
                .run_allowing_failure(&available)
                .map_err(|source| PackageError::Query {
                    manager: self.kind(),
                    package: package.to_owned(),
                    source,
                })?;

        if lookup.code == 0 {
            Ok(PackageState::Available)
        } else {
            Ok(PackageState::Unavailable)
        }
    }

    fn install(
        &self,
        runner: &dyn CommandRunner,
        packages: &[String],
    ) -> Result<InstallOutcome, PackageError> {
        let newly_installed = super::newly_installed(self, runner, packages)?;

        if !newly_installed.is_empty() {
            let install = Cmd::root(DNF)
                .arg(INSTALL)
                .arg("--assumeyes")
                .args(newly_installed.iter().cloned());

            runner
                .run(&install)
                .map_err(|source| PackageError::Install {
                    manager: self.kind(),
                    packages: newly_installed.clone(),
                    source,
                })?;
        }

        Ok(InstallOutcome {
            requested: packages.to_vec(),
            newly_installed,
        })
    }
}

#[cfg(test)]
#[path = "dnf_test.rs"]
mod dnf_test;
