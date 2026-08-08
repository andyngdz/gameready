use super::*;

#[test]
fn every_finding_reads_as_a_phrase_that_follows_a_step_name() {
    let findings = [
        Probe::Applicable,
        Probe::AlreadyApplied {
            evidence: "swappiness is 180".to_owned(),
        },
        Probe::NotApplicable {
            reason: "Arch ships scx".to_owned(),
        },
        Probe::Conflict {
            with: "tuned.service".to_owned(),
            detail: "tuned.service sets the governor on its own schedule".to_owned(),
            yours: None,
        },
        Probe::Unknown {
            reason: "github.com timed out".to_owned(),
        },
    ];

    for finding in &findings {
        let described = finding.describe();
        // A bracket after a step name reads as an aside, and what probing found
        // is the whole point of the line.
        assert!(!described.contains('('), "{described}");
        assert!(
            described.starts_with(|first: char| first.is_lowercase()),
            "{described}"
        );
    }
}

#[test]
fn a_conflict_says_what_owns_the_setting_rather_than_that_there_is_one() {
    let conflict = Probe::Conflict {
        with: "power-profiles-daemon".to_owned(),
        detail: "power-profiles-daemon resets the governor on its own schedule".to_owned(),
        yours: Some("systemctl disable --now power-profiles-daemon".to_owned()),
    };

    assert_eq!(
        conflict.describe(),
        "power-profiles-daemon resets the governor on its own schedule"
    );
}
