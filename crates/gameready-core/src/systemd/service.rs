//! Asking systemctl about one unit.

use crate::exec::{Cmd, CommandRunner};
use crate::systemd::constants::{ACTIVE, ENABLED_ANSWERS, IS_ACTIVE, IS_ENABLED, SYSTEMCTL};
use crate::systemd::domain::UnitState;
use crate::systemd::errors::SystemdError;

/// Reads where one unit stands.
///
/// Both queries run unprivileged and neither touches the unit. `is-enabled`
/// goes first because it is also the existence check: a name with no unit file
/// answers nothing, and asking `is-active` about it would print `inactive` and
/// make a missing package look like an installed-but-stopped one.
pub fn unit_state(runner: &dyn CommandRunner, unit: &str) -> Result<UnitState, SystemdError> {
    if runner.which(SYSTEMCTL).is_none() {
        return Err(SystemdError::Unavailable);
    }

    let enabled = ask(runner, IS_ENABLED, unit)?;
    if enabled.is_empty() {
        return Ok(UnitState::Absent);
    }

    if ask(runner, IS_ACTIVE, unit)? == ACTIVE {
        return Ok(UnitState::Running);
    }

    if ENABLED_ANSWERS.contains(&enabled.as_str()) {
        Ok(UnitState::EnabledNotStarted)
    } else {
        Ok(UnitState::Dormant)
    }
}

/// Runs one `systemctl` question and returns its trimmed answer.
///
/// Failure is allowed: `is-enabled` on a disabled unit and `is-active` on a
/// stopped one both exit non-zero while printing the answer that was asked for.
fn ask(runner: &dyn CommandRunner, question: &str, unit: &str) -> Result<String, SystemdError> {
    let query = Cmd::user(SYSTEMCTL).arg(question).arg(unit);
    let output = runner
        .run_allowing_failure(&query)
        .map_err(|source| SystemdError::Query {
            unit: unit.to_owned(),
            source,
        })?;
    Ok(output.stdout_trimmed().to_owned())
}

#[cfg(test)]
#[path = "service_test.rs"]
mod service_test;
