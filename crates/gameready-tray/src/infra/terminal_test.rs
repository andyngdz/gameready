use std::sync::Mutex;

use gameready_core::infra::exec::MockRunner;

use super::*;

/// What a fake spawn sees, as plain strings, for asserting argv.
fn captured(cmd: &Command) -> Vec<String> {
    let mut argv = vec![cmd.get_program().to_string_lossy().into_owned()];
    argv.extend(cmd.get_args().map(|arg| arg.to_string_lossy().into_owned()));
    argv
}

#[test]
fn resolves_the_system_default_terminal_first() {
    let runner = MockRunner::new()
        .with_binary("xdg-terminal-exec")
        .with_binary("gnome-terminal");

    let launch = resolve(&|binary| runner.which(binary)).expect("a terminal resolves");

    assert_eq!(launch.program, PathBuf::from("/usr/bin/xdg-terminal-exec"));
    assert_eq!(launch.style, ArgStyle::Exec);
}

#[test]
fn falls_back_to_x_terminal_emulator_without_the_freedesktop_helper() {
    let runner = MockRunner::new().with_binary("x-terminal-emulator");

    let launch = resolve(&|binary| runner.which(binary)).expect("a terminal resolves");

    assert_eq!(
        launch.program,
        PathBuf::from("/usr/bin/x-terminal-emulator")
    );
    assert_eq!(launch.style, ArgStyle::DashE);
}

#[test]
fn consults_the_detection_list_in_order() {
    let runner = MockRunner::new()
        .with_binary("konsole")
        .with_binary("xterm");

    let launch = resolve(&|binary| runner.which(binary)).expect("a terminal resolves");

    assert_eq!(launch.program, PathBuf::from("/usr/bin/konsole"));
    assert_eq!(launch.style, ArgStyle::DashE);
}

#[test]
fn no_terminal_resolves_to_none() {
    let runner = MockRunner::new();

    assert_eq!(resolve(&|binary| runner.which(binary)), None);
}

#[test]
fn every_arg_style_passes_the_gameready_command_through() {
    let exec = Launch {
        program: PathBuf::from("/usr/bin/xdg-terminal-exec"),
        style: ArgStyle::Exec,
    };
    assert_eq!(
        captured(&command(&exec, PathBuf::from("/usr/bin/gameready"))),
        [
            "/usr/bin/xdg-terminal-exec",
            "/usr/bin/gameready",
            "apply",
            "--step",
            PROTON_GE_STEP_ID,
            "--yes",
        ]
    );

    let dash_e = Launch {
        program: PathBuf::from("/usr/bin/konsole"),
        style: ArgStyle::DashE,
    };
    assert_eq!(
        captured(&command(&dash_e, PathBuf::from("/usr/bin/gameready"))),
        [
            "/usr/bin/konsole",
            "-e",
            "/usr/bin/gameready",
            "apply",
            "--step",
            PROTON_GE_STEP_ID,
            "--yes",
        ]
    );

    let double_dash = Launch {
        program: PathBuf::from("/usr/bin/gnome-terminal"),
        style: ArgStyle::DoubleDash,
    };
    assert_eq!(
        captured(&command(&double_dash, PathBuf::from("/usr/bin/gameready"))),
        [
            "/usr/bin/gnome-terminal",
            "--",
            "/usr/bin/gameready",
            "apply",
            "--step",
            PROTON_GE_STEP_ID,
            "--yes",
        ]
    );
}

#[test]
fn launch_with_hands_the_built_command_to_the_spawner() {
    let runner = MockRunner::new()
        .with_binary("gameready")
        .with_binary("xdg-terminal-exec");
    let spawned = Mutex::new(None::<Vec<String>>);

    let result = launch_with(&runner, &|cmd| {
        *spawned.lock().unwrap() = Some(captured(cmd));
        Ok(())
    });

    assert!(result.is_ok());
    assert_eq!(
        *spawned.lock().unwrap(),
        Some(vec![
            "/usr/bin/xdg-terminal-exec".to_owned(),
            "/usr/bin/gameready".to_owned(),
            "apply".to_owned(),
            "--step".to_owned(),
            PROTON_GE_STEP_ID.to_owned(),
            "--yes".to_owned(),
        ])
    );
}

#[test]
fn launch_with_refuses_before_resolving_a_terminal_when_gameready_is_missing() {
    let runner = MockRunner::new().with_binary("xdg-terminal-exec");

    let result = launch_with(&runner, &|_cmd| Ok(()));

    assert!(matches!(result, Err(TerminalError::GamereadyNotFound)));
}

#[test]
fn launch_with_refuses_when_no_terminal_exists() {
    let runner = MockRunner::new().with_binary("gameready");

    let result = launch_with(&runner, &|_cmd| Ok(()));

    assert!(matches!(result, Err(TerminalError::NoTerminal)));
}

#[test]
fn launch_with_names_the_terminal_when_it_cannot_start() {
    let runner = MockRunner::new()
        .with_binary("gameready")
        .with_binary("konsole");

    let result = launch_with(&runner, &|_cmd| Err(std::io::Error::other("no display")));

    match result {
        Err(TerminalError::Spawn { program, .. }) => {
            assert_eq!(program, "/usr/bin/konsole");
        }
        other => panic!("expected Spawn, got {other:?}"),
    }
}
