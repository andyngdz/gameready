use super::*;

#[test]
fn the_managed_header_names_the_marker_step_and_run() {
    let run = RunId::generate();
    let header = managed_header(ImprovementId::from_static("core.io.scheduler"), run);

    assert!(header.starts_with(MANAGED_HEADER), "{header}");
    assert!(header.contains("step=core.io.scheduler"), "{header}");
    assert!(header.contains(&format!("run={run}")), "{header}");
    // doctor reads a single line; a newline would split the marker.
    assert!(!header.contains('\n'), "{header}");
}
