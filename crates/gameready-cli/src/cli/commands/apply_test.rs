use gameready_core::improvement::ImprovementId;
use gameready_core::steps::find_core_step;

#[test]
fn an_unknown_step_id_is_rejected_before_anything_runs() {
    // apply --step takes a user string; a typo must fail here rather than
    // silently apply the whole catalog.
    let id = ImprovementId::parse("core.does.not-exist").expect("well formed");
    assert!(find_core_step(&id).is_none());
}

#[test]
fn a_malformed_step_id_does_not_parse() {
    assert!(ImprovementId::parse("Core.Sysctl").is_err());
}
