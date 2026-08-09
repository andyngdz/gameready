use super::*;

#[test]
fn a_known_pci_id_names_its_vendor() {
    assert_eq!(
        GpuVendor::from_pci_id("0x10de"),
        DetectedGpu::Recognised(GpuVendor::Nvidia)
    );
    assert_eq!(
        GpuVendor::from_pci_id("0x1002"),
        DetectedGpu::Recognised(GpuVendor::Amd)
    );
    assert_eq!(
        GpuVendor::from_pci_id("0x8086"),
        DetectedGpu::Recognised(GpuVendor::Intel)
    );
}

#[test]
fn a_pci_id_is_read_whatever_case_and_padding_sysfs_used() {
    assert_eq!(
        GpuVendor::from_pci_id("0x10DE\n"),
        DetectedGpu::Recognised(GpuVendor::Nvidia)
    );
}

#[test]
fn an_unrecognised_vendor_is_named_as_such_rather_than_guessed() {
    assert_eq!(GpuVendor::from_pci_id("0x1af4"), DetectedGpu::Unrecognised);
    assert_eq!(GpuVendor::from_pci_id(""), DetectedGpu::Unrecognised);
}

#[test]
fn nvidia_gets_a_bounded_cache_and_never_the_cleanup_bypass() {
    let settings = GpuVendor::Nvidia.cache_settings();

    assert!(settings
        .iter()
        .any(|(key, value)| *key == "__GL_SHADER_DISK_CACHE_SIZE" && value == "12000000000"));
    // SKIP_CLEANUP removes the limit rather than raising it, so a cache set to
    // 12GB alongside it would grow until the disk filled.
    assert!(!settings.iter().any(|(key, _)| key.contains("SKIP_CLEANUP")));
}

#[test]
fn mesa_vendors_get_the_suffixed_size_mesa_parses() {
    for vendor in [GpuVendor::Amd, GpuVendor::Intel] {
        assert_eq!(
            vendor.cache_settings(),
            vec![("MESA_SHADER_CACHE_MAX_SIZE", "12G".to_owned())],
            "{vendor:?}"
        );
    }
}
