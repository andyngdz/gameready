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
fn a_result_nothing_will_come_of_reads_as_inactive_whether_or_not_probing_worked() {
    // Both mean "this row will not change", and a surface that told them apart
    // by colour would be inventing a distinction the user cannot act on.
    let ruled_out = Probe::NotApplicable {
        reason: "kernel has no sched_ext".to_owned(),
    };
    let unreadable = Probe::Unknown {
        reason: "github.com timed out".to_owned(),
    };

    assert_eq!(ruled_out.status(), ProbeStatus::Inactive);
    assert_eq!(unreadable.status(), ProbeStatus::Inactive);
}

#[test]
fn an_applied_step_and_one_that_would_apply_never_share_a_status() {
    // The whole point of the split: a user reading "set" must not be looking at
    // a step that has not run yet.
    let already = Probe::AlreadyApplied {
        evidence: "swappiness is 10".to_owned(),
    };

    assert_eq!(already.status(), ProbeStatus::Set);
    assert_eq!(Probe::Applicable.status(), ProbeStatus::Ready);
}

#[test]
fn a_conflict_is_the_only_status_that_asks_the_user_to_read_something() {
    let conflict = Probe::Conflict {
        with: "tuned.service".to_owned(),
        detail: "tuned.service sets the governor on its own schedule".to_owned(),
        yours: None,
    };

    assert_eq!(conflict.status(), ProbeStatus::Attention);
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
