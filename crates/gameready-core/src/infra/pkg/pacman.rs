//! Arch and its derivatives.

use crate::exec::{Cmd, CommandRunner};
use crate::facts::PackageManagerKind;
use crate::pkg::{InstallOutcome, PackageError, PackageManager, PackageState};

/// Arch's package manager.
const PACMAN: &str = "pacman";

/// Drives `pacman`.
#[derive(Debug, Default, Clone, Copy)]
pub struct Pacman;

impl PackageManager for Pacman {
    fn kind(&self) -> PackageManagerKind {
        PackageManagerKind::Pacman
    }

    fn state(
        &self,
        runner: &dyn CommandRunner,
        package: &str,
    ) -> Result<PackageState, PackageError> {
        // -Q asks the local database only, so an installed package is reported
        // even when its repository has gone away.
        let installed = Cmd::user(PACMAN).arg("-Q").arg(package);
        let query =
            runner
                .run_allowing_failure(&installed)
                .map_err(|source| PackageError::Query {
                    manager: self.kind(),
                    package: package.to_owned(),
                    source,
                })?;

        if query.code == 0 {
            // `pacman -Q foo` prints "foo 1.2.3-1".
            let version = query
                .stdout_trimmed()
                .split_whitespace()
                .nth(1)
                .map(str::to_owned);
            return Ok(PackageState::Installed { version });
        }

        let available = Cmd::user(PACMAN).arg("-Si").arg(package);
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
            // -S without -y: refreshing the database as part of an install is
            // how a partial upgrade happens on Arch, and a partial upgrade is
            // the classic way to break the system.
            let install = Cmd::root(PACMAN)
                .arg("-S")
                .arg("--needed")
                .arg("--noconfirm")
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
#[path = "pacman_test.rs"]
mod pacman_test;
