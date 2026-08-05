use super::*;

#[test]
fn accepts_a_dotted_kebab_case_id() {
    let id = ImprovementId::parse("core.sysctl.max-map-count").expect("well formed");
    assert_eq!(id.as_str(), "core.sysctl.max-map-count");
    assert_eq!(id.namespace(), "core");
}

#[test]
fn rejects_an_empty_id() {
    assert_eq!(ImprovementId::parse(""), Err(ImprovementIdError::Empty));
}

#[test]
fn rejects_a_trailing_dot_because_it_leaves_an_empty_segment() {
    let error = ImprovementId::parse("core.sysctl.").expect_err("empty segment");
    assert!(matches!(error, ImprovementIdError::EmptySegment { .. }));
}

#[test]
fn rejects_uppercase_so_journal_keys_stay_comparable() {
    let error = ImprovementId::parse("core.Sysctl").expect_err("uppercase");
    assert!(matches!(error, ImprovementIdError::Malformed { .. }));
}

#[test]
fn a_static_id_and_a_parsed_id_compare_equal() {
    let from_literal = ImprovementId::from_static("core.sysctl.max-map-count");
    let parsed = ImprovementId::parse("core.sysctl.max-map-count").expect("well formed");
    assert_eq!(from_literal, parsed);
}
