use super::*;
use crate::steps::constants::MANAGED_HEADER;

fn step() -> ImprovementId {
    ImprovementId::from_static("core.gamemode.config")
}

#[test]
fn the_file_carries_the_marker_the_step_and_the_run() {
    let run = RunId::generate();
    let body = contents(step(), run);

    assert!(body.starts_with(MANAGED_HEADER), "{body}");
    assert!(body.contains(&format!("run={run}")), "{body}");
    assert!(body.contains("step=core.gamemode.config"), "{body}");
}

#[test]
fn it_writes_the_general_section_gamemode_reads_renice_from() {
    let body = contents(step(), RunId::generate());
    assert!(body.contains("[general]"), "{body}");
    assert!(body.contains("renice=20"), "{body}");
}

#[test]
fn it_writes_nothing_that_is_already_a_gamemode_default() {
    // Restating a default would leave gameready owning it, so a later gamemode
    // release could not change its mind.
    let body = contents(step(), RunId::generate());
    for already_default in ["ioprio", "inhibit_screensaver", "disable_splitlock"] {
        assert!(!body.contains(already_default), "{already_default}: {body}");
    }
}

#[test]
fn softrealtime_is_left_out_because_mainline_cannot_honour_it() {
    let body = contents(step(), RunId::generate());
    assert!(!body.contains("softrealtime"), "{body}");
}

#[test]
fn the_preview_drops_the_header_and_keeps_the_setting() {
    let preview = preview();
    assert!(!preview.contains(MANAGED_HEADER));
    assert!(preview.contains("[general]"));
    assert!(preview.contains("renice=20"));
}

#[test]
fn sets_renice_recognises_our_own_file_and_nothing_weaker() {
    assert!(sets_renice(&contents(step(), RunId::generate())));
    // gamemode's own default, which renices nothing.
    assert!(!sets_renice("[general]\nrenice=0\n"));
    assert!(!sets_renice(""));
}
