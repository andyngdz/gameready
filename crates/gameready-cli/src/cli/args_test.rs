use super::*;

#[test]
fn doctor_never_mutates() {
    assert!(!Command::Doctor.mutates());
}

#[test]
fn a_dry_run_does_not_mutate() {
    let command = Command::Apply {
        step: None,
        dry_run: true,
    };
    assert!(!command.mutates());
}

#[test]
fn every_mutating_command_is_covered() {
    // A command that changes the system without being listed here never primes
    // the credential cache, so its first privileged call fails. rollback and
    // selftest both shipped with exactly that bug.
    let mutating = [
        Command::Apply {
            step: None,
            dry_run: false,
        },
        Command::Rollback {
            run: None,
            purge_packages: false,
        },
        Command::Selftest { step: None },
    ];

    for command in mutating {
        assert!(command.mutates(), "{command:?} does not prime sudo");
    }
}
