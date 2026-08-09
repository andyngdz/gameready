use super::*;
use crate::steps::constants::MANAGED_HEADER;

fn step() -> ImprovementId {
    ImprovementId::from_static("core.gpu.shader-cache")
}

#[test]
fn the_fragment_carries_the_marker_and_the_run() {
    let run = RunId::generate();
    let body = contents(GpuVendor::Nvidia, step(), run);

    assert!(body.starts_with(MANAGED_HEADER), "{body}");
    assert!(body.contains(&format!("run={run}")), "{body}");
    assert!(body.contains("step=core.gpu.shader-cache"), "{body}");
}

#[test]
fn the_fragment_tells_the_reader_when_it_takes_effect() {
    // Nothing in the write itself makes this apply now, so the file has to say
    // so: a user who checks immediately and sees no change is not being lied to.
    let body = contents(GpuVendor::Amd, step(), RunId::generate());
    assert!(body.contains("next login"), "{body}");
}

#[test]
fn nvidia_gets_its_own_variables_and_mesa_gets_mesas() {
    let nvidia = contents(GpuVendor::Nvidia, step(), RunId::generate());
    assert!(nvidia.contains("__GL_SHADER_DISK_CACHE_SIZE=12000000000"));
    assert!(!nvidia.contains("MESA_"));

    let amd = contents(GpuVendor::Amd, step(), RunId::generate());
    assert!(amd.contains("MESA_SHADER_CACHE_MAX_SIZE=12G"));
    assert!(!amd.contains("__GL_"));
}

#[test]
fn every_line_is_a_comment_or_an_assignment_systemd_can_parse() {
    let body = contents(GpuVendor::Intel, step(), RunId::generate());

    for line in body.lines().filter(|line| !line.trim().is_empty()) {
        assert!(
            line.starts_with('#') || line.contains('='),
            "systemd cannot parse {line:?}"
        );
    }
}

#[test]
fn the_preview_drops_the_header_and_keeps_the_assignments() {
    let preview = preview(GpuVendor::Nvidia);

    assert!(!preview.contains(MANAGED_HEADER));
    assert!(preview.contains("__GL_SHADER_DISK_CACHE=1"));
}

#[test]
fn sets_everything_needs_every_assignment_not_just_one() {
    let full = contents(GpuVendor::Nvidia, step(), RunId::generate());
    assert!(sets_everything(&full, GpuVendor::Nvidia));

    // The cache is enabled but its size is still the 1GB default, which is the
    // whole problem the step exists to fix.
    assert!(!sets_everything(
        "__GL_SHADER_DISK_CACHE=1\n",
        GpuVendor::Nvidia
    ));
    assert!(!sets_everything("", GpuVendor::Nvidia));
}

#[test]
fn a_fragment_written_for_one_vendor_does_not_satisfy_another() {
    let amd = contents(GpuVendor::Amd, step(), RunId::generate());
    assert!(!sets_everything(&amd, GpuVendor::Nvidia));
}
