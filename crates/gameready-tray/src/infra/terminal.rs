//! Opening the one command Proton-GE needs in the system's default terminal.
//!
//! The tray never mutates the machine, so a click's whole job is to hand the
//! command to a terminal the user can watch and stop. This module picks which
//! terminal and builds the argv; it does not wait for the window.

use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use gameready_core::exec::CommandRunner;

use crate::infra::errors::TerminalError;
use crate::tray::PROTON_GE_STEP_ID;

/// The binary the terminal is asked to run, resolved on `PATH`.
const GAMEREADY_BIN: &str = "gameready";

/// The one way to update Proton-GE, shared with the hand-typed `apply --step`.
///
/// Written from the same constant the sweep matches on, so a click and a
/// command can never name a different step.
const GAMEREADY_ARGS: [&str; 4] = ["apply", "--step", PROTON_GE_STEP_ID, "--yes"];

/// How a terminal takes the command that follows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArgStyle {
    /// `xdg-terminal-exec`: the helper takes the command as its own argv.
    Exec,

    /// `-e <command> <args>`, the `konsole` / `alacritty` / `xterm` spelling.
    DashE,

    /// `-- <command> <args>`, the `gnome-terminal` / `kitty` spelling.
    DoubleDash,
}

/// Which terminal was found and how to talk to it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Launch {
    program: PathBuf,
    style: ArgStyle,
}

/// The terminals this tray knows how to open, most preferred first.
///
/// `xdg-terminal-exec` is the freedesktop way to ask for "the default
/// terminal" and hands the rest to the desktop's choice. The list that follows
/// is the fallback when that helper is absent, in roughly how common each
/// emulator is on a desktop running this tray.
const TERMINALS: &[(ArgStyle, &str)] = &[
    (ArgStyle::Exec, "xdg-terminal-exec"),
    (ArgStyle::DashE, "x-terminal-emulator"),
    (ArgStyle::DoubleDash, "gnome-terminal"),
    (ArgStyle::DashE, "konsole"),
    (ArgStyle::DashE, "alacritty"),
    (ArgStyle::DoubleDash, "kitty"),
    (ArgStyle::DashE, "xterm"),
];

/// Opens the default terminal running the Proton-GE update command.
///
/// Both the terminal and the `gameready` binary resolve through
/// `runner.which`, so the tray asks the machine what exists rather than
/// assuming a PATH.
pub fn launch(runner: &dyn CommandRunner) -> Result<(), TerminalError> {
    launch_with(runner, &spawn)
}

/// The `launch` path with the spawn step injectable.
///
/// The tray must not block on the window: the command runs where the user can
/// see it, and the tray has a menu to keep serving. Production drops the child
/// handle; a test swaps this for a fake that records the command.
fn launch_with(
    runner: &dyn CommandRunner,
    spawn: &dyn Fn(&mut Command) -> std::io::Result<()>,
) -> Result<(), TerminalError> {
    let which = |binary: &str| runner.which(binary);
    // A terminal with nothing to run would flash and die, so the binary is
    // checked before any terminal is resolved.
    let Some(gameready) = which(GAMEREADY_BIN) else {
        return Err(TerminalError::GamereadyNotFound);
    };
    let Some(launch) = resolve(&which) else {
        return Err(TerminalError::NoTerminal);
    };
    let mut cmd = command(&launch, gameready);
    spawn(&mut cmd).map_err(|source| TerminalError::Spawn {
        program: launch.program.display().to_string(),
        source,
    })
}

/// The first terminal on this machine, in [`TERMINALS`] order.
fn resolve(which: &dyn Fn(&str) -> Option<PathBuf>) -> Option<Launch> {
    TERMINALS.iter().find_map(|(style, binary)| {
        which(binary).map(|program| Launch {
            program,
            style: *style,
        })
    })
}

/// The command that opens `launch` running `gameready`.
///
/// The terminal's own stdio is nulled because it opens its own window and its
/// own session; the tray keeps no pipe to it. `process_group(0)` makes the
/// window a group leader of its own, so a signal aimed at the tray's group
/// does not take the update with it.
fn command(launch: &Launch, gameready: PathBuf) -> Command {
    let mut cmd = Command::new(&launch.program);
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    cmd.process_group(0);
    match launch.style {
        ArgStyle::Exec => {
            cmd.arg(gameready).args(GAMEREADY_ARGS);
        }
        ArgStyle::DashE => {
            cmd.arg("-e").arg(gameready).args(GAMEREADY_ARGS);
        }
        ArgStyle::DoubleDash => {
            cmd.arg("--").arg(gameready).args(GAMEREADY_ARGS);
        }
    }
    cmd
}

/// Starts the command and drops the handle, so nothing here waits on the
/// terminal window.
fn spawn(cmd: &mut Command) -> std::io::Result<()> {
    cmd.spawn().map(drop)
}

#[cfg(test)]
#[path = "terminal_test.rs"]
mod terminal_test;
