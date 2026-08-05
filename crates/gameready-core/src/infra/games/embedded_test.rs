use super::*;
use crate::games::GameKey;

#[test]
fn every_shipped_profile_parses() {
    // These files ship inside the binary, so a broken one is a bug in this
    // repository and there is no user action that fixes it.
    let (profiles, failures) = builtin_profiles();
    assert!(failures.is_empty(), "{failures:?}");
    assert!(!profiles.is_empty(), "no profiles were embedded");
}

#[test]
fn no_two_shipped_profiles_share_a_key() {
    // Two would silently shadow one another in the catalog.
    let (profiles, _) = builtin_profiles();
    let mut keys: Vec<GameKey> = profiles.iter().map(GameProfile::key).collect();
    let before = keys.len();
    keys.sort();
    keys.dedup();
    assert_eq!(keys.len(), before, "two shipped profiles share a key");
}

#[test]
fn no_two_shipped_profiles_share_an_appid() {
    // A duplicate appid means one game's settings land on another.
    let (profiles, _) = builtin_profiles();
    let mut ids: Vec<_> = profiles.iter().map(|profile| profile.app_id).collect();
    let before = ids.len();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), before, "two shipped profiles share an appid");
}

#[test]
fn the_verified_profiles_are_present_with_their_real_appids() {
    // Read off this machine's Steam library on 2026-08-05. A wrong appid
    // applies one game's settings to another and nothing else catches it.
    let (profiles, _) = builtin_profiles();
    let by_key = |key: &str| {
        profiles
            .iter()
            .find(|profile| profile.key().as_str() == key)
            .map(|profile| profile.app_id.0)
    };

    assert_eq!(by_key("deadlock"), Some(1_422_450));
    assert_eq!(by_key("cyberpunk-2077"), Some(1_091_500));
    assert_eq!(by_key("slay-the-spire-2"), Some(2_868_840));
}
