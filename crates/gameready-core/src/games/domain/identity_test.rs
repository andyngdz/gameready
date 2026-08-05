use super::*;

#[test]
fn a_key_is_lowercase_and_hyphenated() {
    assert_eq!(
        GameKey::from_name("Cyberpunk 2077").as_str(),
        "cyberpunk-2077"
    );
}

#[test]
fn casing_and_spacing_reach_the_same_profile() {
    // The user types this on the command line and the three catalog layers are
    // directories people name by hand, so these must not be three games.
    let canonical = GameKey::from_name("Slay the Spire 2");
    assert_eq!(GameKey::from_name("slay the spire 2"), canonical);
    assert_eq!(GameKey::from_name("Slay  the   Spire  2"), canonical);
    assert_eq!(GameKey::from_name("slay-the-spire-2"), canonical);
}

#[test]
fn punctuation_collapses_rather_than_doubling_up() {
    assert_eq!(
        GameKey::from_name("S.T.A.L.K.E.R. 2: Heart of Chornobyl").as_str(),
        "s-t-a-l-k-e-r-2-heart-of-chornobyl"
    );
}

#[test]
fn a_name_never_starts_or_ends_with_a_separator() {
    assert_eq!(GameKey::from_name("  Deadlock!  ").as_str(), "deadlock");
}

#[test]
fn a_name_of_pure_punctuation_yields_an_empty_key() {
    // Rejected at parse time by GameError::NoName rather than here; this pins
    // down what the key function does with it so the two cannot drift.
    assert_eq!(GameKey::from_name("!!!").as_str(), "");
}

#[test]
fn an_app_id_displays_as_a_bare_number() {
    assert_eq!(AppId(1422450).to_string(), "1422450");
}
