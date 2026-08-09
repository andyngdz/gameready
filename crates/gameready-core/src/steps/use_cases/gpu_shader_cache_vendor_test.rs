use super::*;
use crate::infra::exec::MockRunner;

/// A machine with one card reporting the given PCI vendor id.
fn machine_with_card(vendor_id: &str) -> MockRunner {
    MockRunner::new().with_file(
        "/sys/class/drm/card0/device/vendor",
        format!("{vendor_id}\n"),
    )
}

#[test]
fn an_nvidia_card_is_recognised() {
    let runner = machine_with_card("0x10de");
    assert_eq!(
        detect_vendor(&runner).expect("detects"),
        DetectedGpu::Recognised(GpuVendor::Nvidia)
    );
}

#[test]
fn an_amd_card_is_recognised() {
    let runner = machine_with_card("0x1002");
    assert_eq!(
        detect_vendor(&runner).expect("detects"),
        DetectedGpu::Recognised(GpuVendor::Amd)
    );
}

#[test]
fn a_connector_entry_is_not_mistaken_for_a_card() {
    // card0-DP-1 carries the same device/vendor as card0. Reading connectors
    // would report one GPU several times.
    let runner = MockRunner::new()
        .with_file("/sys/class/drm/card0/device/vendor", "0x1002\n")
        .with_file("/sys/class/drm/card0-DP-1/device/vendor", "0x1002\n");

    assert_eq!(
        detect_vendor(&runner).expect("detects"),
        DetectedGpu::Recognised(GpuVendor::Amd)
    );
}

#[test]
fn a_machine_with_no_drm_nodes_is_unrecognised_rather_than_an_error() {
    // A container, or a headless server. Nothing to configure, and that is not
    // a failure.
    assert_eq!(
        detect_vendor(&MockRunner::new()).expect("detects"),
        DetectedGpu::Unrecognised
    );
}

#[test]
fn a_vendor_with_no_settings_here_is_unrecognised() {
    let runner = machine_with_card("0x1af4");
    assert_eq!(
        detect_vendor(&runner).expect("detects"),
        DetectedGpu::Unrecognised
    );
}

#[test]
fn a_card_without_a_vendor_file_is_skipped_rather_than_failing() {
    let runner = MockRunner::new()
        .with_file("/sys/class/drm/card0/uevent", "DEVNAME=dri/card0\n")
        .with_file("/sys/class/drm/card1/device/vendor", "0x10de\n");

    assert_eq!(
        detect_vendor(&runner).expect("detects"),
        DetectedGpu::Recognised(GpuVendor::Nvidia)
    );
}
