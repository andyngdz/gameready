use super::*;

#[test]
fn doctor_never_mutates() {
    assert_eq!(Command::Doctor.effect(), Effect::Reads);
}

#[test]
fn listing_games_never_mutates() {
    assert_eq!(Command::ListGames.effect(), Effect::Reads);
}

#[test]
fn a_dry_run_does_not_mutate() {
    let command = Command::Apply {
        step: None,
        yes: false,
        dry_run: true,
    };
    assert_eq!(command.effect(), Effect::Reads);
    assert_eq!(command.mode(), Mode::DryRun);
}

#[test]
fn a_dry_run_init_does_not_mutate() {
    let command = Command::Init {
        yes: true,
        fps_overlay: false,
        dry_run: true,
    };
    assert_eq!(command.effect(), Effect::Reads);
    assert_eq!(command.mode(), Mode::DryRun);
}

#[test]
fn every_mutating_command_is_covered() {
    // A command that changes the system without being listed here never primes
    // the credential cache, so its first privileged call fails. rollback and
    // selftest both shipped with exactly that bug.
    let mutating = [
        Command::Init {
            yes: false,
            fps_overlay: false,
            dry_run: false,
        },
        Command::Apply {
            step: None,
            yes: false,
            dry_run: false,
        },
        Command::Rollback {
            run: None,
            purge_packages: false,
        },
        Command::Selftest { step: None },
    ];

    for command in mutating {
        assert_eq!(
            command.effect(),
            Effect::Mutates,
            "{command:?} does not prime sudo"
        );
    }
}

#[test]
fn a_command_without_a_dry_run_flag_always_applies() {
    assert_eq!(Command::Selftest { step: None }.mode(), Mode::Apply);
}
