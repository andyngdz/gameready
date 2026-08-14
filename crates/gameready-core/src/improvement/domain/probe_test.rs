use super::*;

#[test]
fn every_finding_reads_as_a_phrase_that_follows_a_step_name() {
    let findings = [
        Probe::Applicable,
        Probe::AlreadyApplied {
            evidence: "swappiness is 180".to_owned(),
        },
        Probe::UpdateAvailable {
            installed: "GE-Proton11-3".to_owned(),
            latest: "GE-Proton11-5".to_owned(),
        },
        Probe::NotApplicable {
            reason: "Arch already ships mangohud".to_owned(),
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
        reason: "this machine cannot run it".to_owned(),
    };
    let unreadable = Probe::Unknown {
        reason: "github.com timed out".to_owned(),
    };

    assert_eq!(ruled_out.status(), ProbeStatus::Inactive);
    assert_eq!(unreadable.status(), ProbeStatus::Inactive);
}

#[test]
fn an_update_available_step_is_neither_set_nor_would_apply() {
    let update = Probe::UpdateAvailable {
        installed: "GE-Proton11-3".to_owned(),
        latest: "GE-Proton11-5".to_owned(),
    };

    assert_eq!(update.status(), ProbeStatus::UpdateAvailable);
    assert_ne!(update.status(), ProbeStatus::Set);
    assert_ne!(update.status(), ProbeStatus::Ready);
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
fn a_conflict_is_not_attention_because_nothing_is_broken() {
    let conflict = Probe::Conflict {
        with: "tuned.service".to_owned(),
        detail: "tuned.service sets the governor on its own schedule".to_owned(),
        yours: None,
    };

    assert_eq!(conflict.status(), ProbeStatus::Conflict);
    assert_ne!(conflict.status(), ProbeStatus::Attention);
}

#[test]
fn attention_is_reserved_for_a_probe_that_could_not_read_the_machine() {
    // Attention comes from the reading path itself, not from any probe answer:
    // a conflict is a machine state the user chose, and an unknown is a row
    // nothing will come of. `StepFinding` maps a probe error to Attention, and
    // this pins that no probe value wears it.
    let statuses = [
        Probe::Applicable,
        Probe::AlreadyApplied {
            evidence: "swappiness is 10".to_owned(),
        },
        Probe::NotApplicable {
            reason: "this machine cannot run it".to_owned(),
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

    for probe in &statuses {
        assert_ne!(probe.status(), ProbeStatus::Attention, "{probe:?}");
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
