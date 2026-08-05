//! Debian, Ubuntu, and their derivatives.

use crate::exec::{Cmd, CommandRunner};
use crate::facts::PackageManagerKind;
use crate::infra::exec::constants::INSTALL;
use crate::pkg::{InstallOutcome, PackageError, PackageManager, PackageState};

/// The front end to script against.
///
/// `apt` prints "this APT has Super Cow Powers" and warns that it has no stable
/// command line interface. `apt-get` is the one with the contract.
const APT_GET: &str = "apt-get";

/// Answers queries without touching the package database.
const APT_CACHE: &str = "apt-cache";

/// Reads the installed version without consulting any repository.
const DPKG_QUERY: &str = "dpkg-query";

/// Drives `apt-get`.
#[derive(Debug, Default, Clone, Copy)]
pub struct Apt;

impl PackageManager for Apt {
    fn kind(&self) -> PackageManagerKind {
        PackageManagerKind::Apt
    }

    fn state(
        &self,
        runner: &dyn CommandRunner,
        package: &str,
    ) -> Result<PackageState, PackageError> {
        // Installed state first, and from dpkg rather than apt: a package can be
        // installed while its repository is gone, and asking apt would call
        // that unavailable.
        let installed = Cmd::user(DPKG_QUERY)
            .arg("--showformat=${Version}")
            .arg("--show")
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

        let available = Cmd::user(APT_CACHE).arg("show").arg(package);
        let lookup =
            runner
                .run_allowing_failure(&available)
                .map_err(|source| PackageError::Query {
                    manager: self.kind(),
                    package: package.to_owned(),
                    source,
                })?;

        if lookup.code == 0 && !lookup.stdout_trimmed().is_empty() {
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
            let install = Cmd::root(APT_GET)
                .arg(INSTALL)
                .arg("--yes")
                // Recommends pull in a surprising amount on Debian, and a user
                // agreed to a size estimate that did not include them.
                .arg("--no-install-recommends")
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
#[path = "apt_test.rs"]
mod apt_test;
