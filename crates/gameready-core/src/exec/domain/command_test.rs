use super::*;

#[test]
fn renders_a_user_command_as_typed() {
    let cmd = Cmd::user("uname").arg("-r");
    assert_eq!(cmd.to_string(), "uname -r");
    assert!(!cmd.needs_root());
}

#[test]
fn renders_a_root_command_with_its_escalator() {
    let cmd = Cmd::root("sysctl").arg("-w").arg("vm.max_map_count=2147483642");
    assert_eq!(cmd.to_string(), "sudo sysctl -w vm.max_map_count=2147483642");
    assert!(cmd.needs_root());
}

#[test]
fn quotes_arguments_containing_spaces_so_the_line_can_be_pasted() {
    let cmd = Cmd::user("echo").arg("two words");
    assert_eq!(cmd.to_string(), "echo 'two words'");
}

#[test]
fn keeps_arguments_as_a_vector_rather_than_a_shell_string() {
    let cmd = Cmd::user("pacman").args(["-S", "gamemode", "mangohud"]);
    assert_eq!(cmd.arguments(), ["-S", "gamemode", "mangohud"]);
}

#[test]
fn trims_stdout_for_single_value_reads() {
    let output = CmdOutput {
        code: 0,
        stdout: "7.0.0-29-generic\n".to_owned(),
        stderr: String::new(),
    };
    assert_eq!(output.stdout_trimmed(), "7.0.0-29-generic");
}
