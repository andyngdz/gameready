use super::*;

/// What sudo says when the credential cache is cold and `-n` forbids a prompt.
const SUDO_REFUSED: &str = "sudo: interactive authentication is required";

fn write_refused(reason: &str) -> StepError {
    StepError::Exec(ExecError::Write {
        path: PathBuf::from("/etc/sysctl.d/99-gameready.conf"),
        source: std::io::Error::other(reason.to_owned()),
    })
}

#[test]
fn a_write_failure_names_the_reason_the_system_gave() {
    let error = write_refused(SUDO_REFUSED);

    assert!(
        !error.to_string().contains(SUDO_REFUSED),
        "the plain message is why this method exists: {error}"
    );

    let described = error.describe();
    assert!(
        described.contains("99-gameready.conf"),
        "still names the file: {described}"
    );
    assert!(
        described.contains(SUDO_REFUSED),
        "and now names the cause: {described}"
    );
}

#[test]
fn a_command_failure_shows_its_stderr() {
    let error = StepError::Command {
        command: "sha512sum verification of GE-Proton11-3.tar.gz".to_owned(),
        code: 1,
        stderr: "expected abc123, got def456".to_owned(),
    };

    let described = error.describe();
    assert!(
        described.contains("expected abc123, got def456"),
        "the field was captured and never rendered: {described}"
    );
}

#[test]
fn a_refused_command_is_not_reported_twice() {
    let error = StepError::Exec(ExecError::NonZeroExit {
        command: "sudo -n rm -f /etc/sysctl.d/99-gameready.conf".to_owned(),
        code: 1,
        stdout: String::new(),
        stderr: SUDO_REFUSED.to_owned(),
    });

    let described = error.describe();
    assert_eq!(
        described.matches(SUDO_REFUSED).count(),
        1,
        "transparent forwards Display and source to the same error: {described}"
    );
}

#[test]
fn stderr_newlines_do_not_break_the_one_line_summary() {
    let error = write_refused(indoc::indoc! {"
        first line
        second line
    "});

    let described = error.describe();
    assert!(
        !described.contains('\n'),
        "a summary row cannot take newlines: {described:?}"
    );
    assert!(
        described.contains("first line second line"),
        "and keeps both lines: {described}"
    );
}

#[test]
fn a_verification_failure_has_no_cause_to_add() {
    let error = StepError::VerificationFailed {
        step: ImprovementId::from_static("core.sysctl.max-map-count"),
        failed: 1,
        total: 2,
    };

    assert_eq!(error.describe(), error.to_string());
}
