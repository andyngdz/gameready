//! Whether this user is in the group gamemode requires before it will renice.

use crate::exec::{Cmd, CommandRunner};
use crate::improvement::StepError;

/// The group gamemode checks before it renices a client.
const GAMEMODE_GROUP: &str = "gamemode";

/// The command that adds the invoking user to it.
///
/// Shown to the user rather than run: adding yourself to a group needs root,
/// and the change only reaches a process after a fresh login, so gameready
/// cannot make it take effect inside its own run.
pub const JOIN_GROUP: &str = "sudo usermod -aG gamemode $(whoami)";

/// Whether the invoking user's session carries the gamemode group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GamemodeGroup {
    /// The session has it, so gamemode will honour a renice setting.
    Present,

    /// It does not. Either the user was never added, or they were added and
    /// have not logged back in, and gamemode cannot tell those apart either.
    Absent,
}

/// Reads the groups of the running session.
///
/// `id -nG` and not `/etc/group`, because gamemoded checks the credentials the
/// client process actually carries. A user added to the group an hour ago who
/// has not logged out since is in `/etc/group` and still would not get the
/// renice, and reporting them as ready would promise something that does not
/// happen.
pub fn in_gamemode_group(runner: &dyn CommandRunner) -> Result<GamemodeGroup, StepError> {
    let listed = Cmd::user("id").arg("-nG");
    let Ok(output) = runner.run_allowing_failure(&listed) else {
        return Ok(GamemodeGroup::Absent);
    };

    if output
        .stdout
        .split_whitespace()
        .any(|group| group == GAMEMODE_GROUP)
    {
        return Ok(GamemodeGroup::Present);
    }
    Ok(GamemodeGroup::Absent)
}

#[cfg(test)]
#[path = "gamemode_config_group_test.rs"]
mod gamemode_config_group_test;
