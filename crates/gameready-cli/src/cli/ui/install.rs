//! Asking whether to install anything, before the run installs it.

use std::fmt;

use anyhow::Result;
use console::style;
use gameready_core::facts::PackageManagerKind;
use gameready_core::run::{InstallConsent, Mode, PlannedInstall, RunPlan};
use inquire::Select;

use crate::cli::ui::layout::{Mark, Section};
use crate::cli::ui::questions::Picker;
use crate::cli::ui::theme;

/// The one consequence a rollback cannot take back, said where the user agrees
/// to it rather than in the summary afterwards.
const NOT_UNDONE: &str =
    "Rollback restores your config, but leaves packages installed. Removing them is your call, \
     not mine.";

/// What saying no costs, since it is not the whole run.
const DECLINING: &str =
    "Say no and the steps that need them stand down. Everything else still runs.";

/// The keys, in the order a user reaches for them.
const KEYS: &str = "↑↓ move · enter confirm · esc installs nothing";

/// The two answers to the install question.
#[derive(Clone, Copy)]
enum Take {
    /// Install nothing, and let the steps that needed it stand down.
    NotNow,
    /// Go ahead.
    Install,
}

impl fmt::Display for Take {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NotNow => "Not now, skip the steps that need them",
            Self::Install => "Install them",
        })
    }
}

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
        let noun = if count == 1 { "package" } else { "packages" };
        if bytes == 0 {
            return format!("{count} {noun} to install");
        }
        format!(
            "{count} {noun} to install {}",
            style(format!("· {}", approx_size(bytes))).yellow()
        )
    }

    /// One package: what it is called and how big, then what it is and what it
    /// is for, each in the step's own words.
    fn package<W: fmt::Write>(s: &mut Section<'_, W>, install: &PlannedInstall) -> fmt::Result {
        let size = if install.approx_bytes == 0 {
            String::new()
        } else {
            let approx = approx_size(install.approx_bytes);
            format!(" {}", style(format!("· {approx}")).dim())
        };
        s.quoted(&format!("{}{size}", style(&install.package).bold()))?;
        s.quoted(&style(&install.what).dim().to_string())?;
        s.quoted(&style(&install.why).dim().to_string())?;
        s.blank()
    }

    /// The question, agreeing with however many packages there are.
    fn question(&self) -> String {
        let them = if self.installs.len() == 1 {
            "it"
        } else {
            "them"
        };
        format!("Install {them}?")
    }

    /// Shows the list and asks whether to go ahead.
    ///
    /// The list is printed rather than carried as the question, because a
    /// prompt whose message runs a dozen lines leaves a dozen blank ones behind
    /// once it is answered. Printed, it stays on screen as the record of what
    /// was agreed to.
    ///
    /// Defaults to no, and an escaped prompt is a no. Every other change
    /// gameready makes comes back off with `gameready rollback`; an installed
    /// package does not, so this is the one question whose safe answer is to do
    /// nothing.
    fn ask(&self) -> Result<InstallConsent> {
        if self.installs.is_empty() {
            return Ok(InstallConsent::Declined);
        }
        if console::user_attended_stderr() {
            // The blank line belongs to the question rather than to the list:
            // the dry run prints the same list with its own screen after it.
            let screen = format!("{self}\n");
            eprint!("{screen}");
        }

        let answer = Select::new(
            &theme::asked(&self.question(), DECLINING),
            vec![Take::NotNow, Take::Install],
        )
        .with_render_config(theme::questions())
        .with_help_message(KEYS)
        .prompt_skippable()?;

        Ok(match answer {
            Some(Take::Install) => InstallConsent::Granted,
            Some(Take::NotNow) | None => InstallConsent::Declined,
        })
    }
}

impl fmt::Display for InstallList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.installs.is_empty() {
            return Ok(());
        }

        let mut s = Section::new(f);
        s.blank()?;
        s.title(&self.headline())?;
        for install in &self.installs {
            Self::package(&mut s, install)?;
        }

        if !self.present.is_empty() {
            s.indented(
                &style(format!("Already here: {}", self.present.join(", ")))
                    .dim()
                    .to_string(),
            )?;
            s.blank()?;
        }

        s.marked(Mark::Warning, &style(NOT_UNDONE).dim().to_string())
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
///
/// Shared with the plan screen, which totals the same packages on one line.
pub(super) fn approx_size(bytes: u64) -> String {
    const MB: u64 = 1_000_000;
    if bytes < MB {
        return "under 1 MB".to_owned();
    }
    format!("{} MB", bytes / MB)
}

#[cfg(test)]
#[path = "install_test.rs"]
mod install_test;
