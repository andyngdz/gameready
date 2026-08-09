use super::*;

fn unreadable() -> StepError {
    StepError::Command {
        command: "systemctl is-active gamemoded".to_owned(),
        code: 4,
        stderr: "Failed to connect to bus".to_owned(),
    }
}

fn finding(found: Result<Probe, StepError>, would_do: Option<&str>) -> StepFinding {
    StepFinding {
        short_name: "shader cache".to_owned(),
        found,
        would_do: would_do.map(ToOwned::to_owned),
    }
}

fn applied() -> Result<Probe, StepError> {
    Ok(Probe::AlreadyApplied {
        evidence: "99-gameready-shader-cache.conf already sets it".to_owned(),
    })
}

#[test]
fn the_full_note_carries_both_halves_when_there_are_two() {
    let finding = finding(Ok(Probe::Applicable), Some("vm.swappiness 60 -> 10"));

    assert_eq!(finding.note(), "would apply, vm.swappiness 60 -> 10");
}

#[test]
fn a_probe_that_could_not_run_asks_for_attention_rather_than_going_grey() {
    let finding = finding(Err(unreadable()), None);

    assert_eq!(finding.status(), ProbeStatus::Attention);
    // The words still name the failure, for the screen that has room for them.
    assert!(finding.note().starts_with("probe failed:"), "{finding:?}");
}

#[test]
fn the_status_of_a_readable_probe_is_the_probes_own() {
    assert_eq!(finding(applied(), None).status(), ProbeStatus::Set);
    assert_eq!(
        finding(Ok(Probe::Applicable), None).status(),
        ProbeStatus::Ready
    );
}
