//! Asking whether to install anything, before the run installs it.

use std::fmt;

use anyhow::Result;
use console::style;
use gameready_core::facts::PackageManagerKind;
use gameready_core::run::{InstallConsent, Mode, PlannedInstall, RunPlan};
use inquire::Confirm;

use crate::cli::ui::questions::Picker;

/// What a run would put on this machine, laid out before it puts any of it
/// there.
pub struct InstallList {
    installs: Vec<PlannedInstall>,
    present: Vec<String>,
}

impl InstallList {
    #[must_use]
    pub fn new(plan: &RunPlan, packages: PackageManagerKind) -> Self {
        Self {
            installs: plan.installs(packages),
            present: plan.already_present(packages),
        }
    }

    /// The count, and the size when anything reported one.
    ///
    /// A package whose step wrote no estimate contributes nothing, so a run
    /// with only those would otherwise read "about 0 MB", which is worse than
    /// saying nothing.
    fn headline(&self) -> String {
        let bytes: u64 = self
            .installs
            .iter()
            .map(|install| install.approx_bytes)
            .sum();
        let count = self.installs.len();
        if bytes == 0 {
            return format!("{count} to install");
        }
        format!("{count} to install, {}", approx_size(bytes))
    }

    /// Shows the list and asks whether to go ahead.
    ///
    /// Defaults to no, and an escaped prompt is a no. Every other change
    /// gameready makes comes back off with `gameready rollback`; an installed
    /// package does not, so this is the one question whose safe answer is to do
    /// nothing.
    fn ask(&self) -> Result<InstallConsent> {
        if self.installs.is_empty() {
            return Ok(InstallConsent::Declined);
        }

        let answer = Confirm::new(&format!("{self}\n\nInstall them?"))
            .with_default(false)
            .with_help_message("no skips the steps that needed them and runs the rest")
            .prompt_skippable()?;

        Ok(match answer {
            Some(true) => InstallConsent::Granted,
            Some(false) | None => InstallConsent::Declined,
        })
    }
}

impl fmt::Display for InstallList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.installs.is_empty() {
            return Ok(());
        }

        writeln!(f, "\n{}\n", self.headline())?;

        for install in &self.installs {
            writeln!(
                f,
                "  {} {}",
                style("*").green(),
                style(&install.package).bold()
            )?;
            writeln!(f, "      What  {}", install.what)?;
            writeln!(f, "      For   {}\n", install.why)?;
        }

        if !self.present.is_empty() {
            writeln!(
                f,
                "  {}",
                style(format!("Already here: {}", self.present.join(", "))).dim()
            )?;
        }

        // The one consequence a user cannot undo with `gameready rollback`, so
        // it belongs on the screen where they agree to it rather than in the
        // summary afterwards.
        writeln!(
            f,
            "\n  {}",
            style("Rollback puts config back but leaves packages installed.").dim()
        )
    }
}

/// Whether this run may install anything.
///
/// The single place that decides, so `init` and `apply` cannot drift apart on
/// the one question that puts software on a user's machine. A dry run installs
/// nothing whatever the answer, so asking would be theatre; `--yes` is the user
/// answering in advance, which is how a scripted run gets its packages without
/// a prompt nobody is there to answer.
pub fn consent_to_install(
    plan: &RunPlan,
    packages: PackageManagerKind,
    picker: Picker,
    mode: Mode,
) -> Result<InstallConsent> {
    if !mode.mutates() {
        return Ok(InstallConsent::Declined);
    }
    match picker {
        Picker::TakeAll => Ok(InstallConsent::Granted),
        Picker::Ask => InstallList::new(plan, packages).ask(),
    }
}

/// Rough download size in the units a user thinks in.
///
/// Hand-rolled rather than pulled from a formatting crate: the input is already
/// an estimate written by hand in each step's `PackageSpec`, so decimal-versus-
/// binary precision would be false accuracy.
fn approx_size(bytes: u64) -> String {
    const MB: u64 = 1_000_000;
    if bytes < MB {
        return "under 1 MB".to_owned();
    }
    format!("about {} MB", bytes / MB)
}

#[cfg(test)]
#[path = "install_test.rs"]
mod install_test;
