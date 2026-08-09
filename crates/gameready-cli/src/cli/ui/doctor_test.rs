use super::*;
use gameready_core::doctor::{MachineReport, StepFinding};
use gameready_core::facts::{Family, SystemFacts};
use gameready_core::improvement::Probe;

fn plain(text: &str) -> String {
    console::strip_ansi_codes(text).into_owned()
}

#[test]
fn it_names_the_machine_and_every_tuning() {
    let facts = SystemFacts::fixture(Family::Arch);
    let machine = MachineReport {
        sched_ext_ready: true,
        swap: None,
        disks: Vec::new(),
    };
    let findings = vec![StepFinding {
        short_name: "CPU governor".to_owned(),
        found: Ok(Probe::Applicable),
        would_do: Some("pin every CPU policy to performance".to_owned()),
    }];

    let rendered = plain(&DoctorReport::new(&facts, &machine, &findings, &[]).to_string());

    assert!(rendered.contains("Your machine"));
    assert!(rendered.contains("sched_ext ready"));
    assert!(rendered.contains("What each tuning would do here"));
    assert!(rendered.contains("CPU governor"));
    assert!(rendered.contains("Nothing above has been changed"));
}

#[test]
fn a_step_that_would_run_says_what_it_would_do_rather_than_that_it_would() {
    // "would apply" on its own is the screen having the answer and keeping it.
    let facts = SystemFacts::fixture(Family::Arch);
    let machine = MachineReport {
        sched_ext_ready: true,
        swap: None,
        disks: Vec::new(),
    };
    let findings = vec![StepFinding {
        short_name: "vm.max_map_count".to_owned(),
        found: Ok(Probe::Applicable),
        would_do: Some("vm.max_map_count 65530 -> 2147483642".to_owned()),
    }];

    let rendered = plain(&DoctorReport::new(&facts, &machine, &findings, &[]).to_string());

    assert!(
        rendered.contains("would apply, vm.max_map_count 65530 -> 2147483642"),
        "{rendered}"
    );
}

#[test]
fn a_step_that_is_already_set_is_not_asked_what_it_would_do() {
    let facts = SystemFacts::fixture(Family::Arch);
    let machine = MachineReport {
        sched_ext_ready: true,
        swap: None,
        disks: Vec::new(),
    };
    let findings = vec![StepFinding {
        short_name: "Swappiness".to_owned(),
        found: Ok(Probe::AlreadyApplied {
            evidence: "swappiness is 180".to_owned(),
        }),
        would_do: None,
    }];

    let rendered = plain(&DoctorReport::new(&facts, &machine, &findings, &[]).to_string());

    let row = rendered
        .lines()
        .find(|line| line.contains("Swappiness"))
        .expect("the row");
    assert!(row.contains("already set, swappiness is 180"), "{rendered}");
}

#[test]
fn a_warning_says_what_it_is_why_it_matters_and_what_to_run() {
    let facts = SystemFacts::fixture(Family::Arch);
    let machine = MachineReport {
        sched_ext_ready: true,
        swap: None,
        disks: Vec::new(),
    };
    let warnings = vec![gameready_core::doctor::Warning {
        finding: "power-profiles-daemon is running".to_owned(),
        explanation: "It resets the CPU governor on its own schedule, so gamemode's changes get \
                      overwritten seconds later."
            .to_owned(),
        suggestion: "systemctl disable --now power-profiles-daemon".to_owned(),
    }];

    let rendered = plain(&DoctorReport::new(&facts, &machine, &[], &warnings).to_string());

    assert!(rendered.contains("Worth knowing"), "{rendered}");
    assert!(
        rendered.contains("! power-profiles-daemon is running"),
        "{rendered}"
    );
    assert!(
        rendered.contains("Fix systemctl disable --now power-profiles-daemon"),
        "{rendered}"
    );
}

#[test]
fn a_kernel_without_sched_ext_says_so() {
    let facts = SystemFacts::fixture(Family::Arch);
    let machine = MachineReport {
        sched_ext_ready: false,
        swap: None,
        disks: Vec::new(),
    };
    let rendered = plain(&DoctorReport::new(&facts, &machine, &[], &[]).to_string());
    assert!(rendered.contains("no sched_ext"));
}
