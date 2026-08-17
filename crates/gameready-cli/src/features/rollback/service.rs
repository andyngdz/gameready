//! Build rollback plans and the rows the terminal preview renders.

use std::path::Path;

use anyhow::{Context as _, Result};
use gameready_core::exec::CommandRunner;
use gameready_core::journal::{self, PriorUnitState, RunId, StatePaths, Undo};
use gameready_core::rollback::{latest_run, plan, PlannedUndo, RollbackError, RollbackPlan};
use gameready_core::steam::{capture_block, PriorBlock, PriorSection};

const EMPTY_VALUE: &str = "(empty)";

/// One line of the rollback preview, ready for the terminal renderer.
pub(crate) struct PreviewRow {
    /// What the undo touches, named for how a user talks about it.
    pub(crate) subject: String,
    /// What will happen, with the current value when it can be read.
    pub(crate) evidence: String,
}

/// Loads and plans the requested journal run.
pub(crate) fn rollback_plan(paths: &StatePaths, requested: Option<&str>) -> Result<RollbackPlan> {
    let records = journal::load(&paths.journal()).context("could not read the journal")?;
    let target = requested
        .map(|text| {
            RunId::parse(text).ok_or(RollbackError::MalformedRun {
                requested: text.to_owned(),
            })
        })
        .transpose()?
        .or_else(|| latest_run(&records))
        .ok_or(RollbackError::NothingRecorded)?;
    Ok(plan(&records, target)?)
}

/// Converts every planned undo into the rows shown before rollback starts.
pub(crate) fn preview_rows(planned: &PlannedUndo, runner: &dyn CommandRunner) -> Vec<PreviewRow> {
    let subject = |evidence: String| PreviewRow {
        subject: planned.undo.subject(),
        evidence,
    };
    match &planned.undo {
        Undo::RestoreSteamConfig { path, sections } => sections
            .iter()
            .map(|section| steam_row(runner, path, section))
            .collect(),
        Undo::SetSysctl { key, value } => vec![subject(sysctl_arrow(runner, key, value))],
        Undo::WriteSysfs { path, value } => {
            vec![subject(arrow(
                current(runner, path).as_deref(),
                Some(value),
            ))]
        }
        Undo::RestoreUnit { unit, prior } => {
            let action = match prior {
                PriorUnitState::WasEnabled => format!("restart {unit} on its own config"),
                PriorUnitState::WasDisabled => format!("disable {unit}"),
            };
            vec![subject(action)]
        }
        Undo::ReportPackages { installed, .. } => {
            vec![subject(format!("keep installed: {}", installed.join(", ")))]
        }
        Undo::DeleteFile { path, .. } | Undo::RemoveDirTree { path, .. } => {
            vec![subject(format!("remove {}", path.display()))]
        }
        Undo::RemoveDirIfEmpty { path, .. } => {
            vec![subject(format!("remove {} if it is empty", path.display()))]
        }
    }
}

/// Builds a row for one Steam config block.
fn steam_row(runner: &dyn CommandRunner, path: &Path, section: &PriorSection) -> PreviewRow {
    let subject = match steam_app(&section.section) {
        Some(id) => format!("{} · {id}", steam_label(section)),
        None => steam_label(section).to_owned(),
    };
    let target = match &section.prior {
        PriorBlock::Absent => None,
        PriorBlock::Present { entries } => entries.first().and_then(|entry| entry.value.clone()),
    };
    PreviewRow {
        subject,
        evidence: arrow(
            steam_current(runner, path, section).as_deref(),
            target.as_deref(),
        ),
    }
}

/// Reads the current value for a recorded Steam key.
fn steam_current(
    runner: &dyn CommandRunner,
    path: &Path,
    section: &PriorSection,
) -> Option<String> {
    let key = match &section.prior {
        PriorBlock::Present { entries } => entries.first()?.key.as_str(),
        PriorBlock::Absent => return None,
    };
    let text = runner.read_to_string(path).ok()?;
    let visit: Vec<&str> = section.section.iter().map(String::as_str).collect();
    match capture_block(&text, &visit, &[key]).ok()? {
        PriorBlock::Present { entries } => entries.first().and_then(|entry| entry.value.clone()),
        PriorBlock::Absent => None,
    }
}

/// Labels a Steam block by the kind of setting it contains.
fn steam_label(section: &PriorSection) -> &'static str {
    if section
        .section
        .iter()
        .any(|part| part == "CompatToolMapping")
    {
        "Proton pin"
    } else if section.section.iter().any(|part| part == "apps") {
        "Launch options"
    } else {
        "Steam setting"
    }
}

/// Returns the app id a per-game Steam block sits under.
fn steam_app(section: &[String]) -> Option<&str> {
    let last = section.last()?;
    (!last.is_empty() && last.chars().all(|character| character.is_ascii_digit()))
        .then_some(last.as_str())
}

/// Reads the current sysctl value and formats its planned restoration.
fn sysctl_arrow(runner: &dyn CommandRunner, key: &str, target: &str) -> String {
    let path = Path::new("/proc/sys").join(key.replace('.', "/"));
    arrow(current(runner, &path).as_deref(), Some(target))
}

/// Reads and trims a file's current contents.
fn current(runner: &dyn CommandRunner, path: &Path) -> Option<String> {
    runner
        .read_to_string(path)
        .ok()
        .map(|text| text.trim().to_owned())
}

/// Formats a current value and its planned target.
fn arrow(current: Option<&str>, target: Option<&str>) -> String {
    let target = target.map_or_else(|| EMPTY_VALUE.to_owned(), shown);
    match current {
        Some(current) => format!("{} → {target}", shown(current)),
        None => format!("→ {target}"),
    }
}

/// Shows blank values explicitly in the preview.
fn shown(value: &str) -> String {
    if value.is_empty() {
        EMPTY_VALUE.to_owned()
    } else {
        value.to_owned()
    }
}

#[cfg(test)]
#[path = "service_test.rs"]
mod service_test;
