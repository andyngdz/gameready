//! Renders and reads back the gamemode.ini the config step owns.

use crate::improvement::ImprovementId;
use crate::journal::RunId;
use crate::steps::constants::managed_header;

/// How far gamemode may raise a client's priority.
///
/// gamemode negates this and applies it as a nice value, so 20 is nice -20, the
/// most the kernel allows. Its own default is 0, which renices nothing at all:
/// installing gamemode and leaving the file alone buys no priority change.
pub const RENICE: u8 = 20;

/// The file's contents, carrying the marker `doctor` looks for.
///
/// Only `renice` is written. Every other setting worth having is already
/// gamemode's own default: `ioprio` is BE/0, `inhibit_screensaver` and
/// `disable_splitlock` are on, and `softrealtime` needs a SCHED_ISO patch
/// mainline does not carry. Writing those would restate the defaults and then
/// leave gameready owning them, so a later gamemode release could not change
/// its mind.
#[must_use]
pub fn contents(step: ImprovementId, run: RunId) -> String {
    format!(
        "{header}\n\
         # Remove this file or run `gameready rollback` to revert.\n\
         [general]\n\
         # Negated and applied as a nice value, so this is nice -{RENICE}.\n\
         {assignment}\n",
        header = managed_header(step, run),
        assignment = assignment(),
    )
}

/// Just the setting, for the plan screen where the header is noise.
#[must_use]
pub fn preview() -> String {
    format!("[general]\n{}", assignment())
}

/// Whether the given file body already carries the setting.
#[must_use]
pub fn sets_renice(body: &str) -> bool {
    body.contains(&assignment())
}

/// The one line gamemode reads out of all this.
fn assignment() -> String {
    format!("renice={RENICE}")
}

#[cfg(test)]
#[path = "gamemode_config_file_test.rs"]
mod gamemode_config_file_test;
