use super::*;
use gameready_core::doctor::MachineReport;
use gameready_core::facts::{Family, SystemFacts};

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
    }];

    let rendered = plain(&DoctorReport::new(&facts, &machine, &findings, &[]).to_string());

    assert!(rendered.contains("Your machine"));
    assert!(rendered.contains("sched_ext ready"));
    assert!(rendered.contains("What each tuning would do here"));
    assert!(rendered.contains("CPU governor"));
    assert!(rendered.contains("Nothing above has been changed"));
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
