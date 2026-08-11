use super::*;

#[test]
fn no_two_statuses_are_drawn_in_the_same_colour() {
    // Set and UpdateAvailable deliberately share the live green, which the
    // sibling test pins; the statuses that must read apart still draw apart.
    let inks = [
        Ink::for_status(ProbeStatus::Set),
        Ink::for_status(ProbeStatus::Ready),
        Ink::for_status(ProbeStatus::Attention),
        Ink::for_status(ProbeStatus::Inactive),
    ];

    let unique = {
        let mut rgb: Vec<_> = inks.iter().map(|ink| ink.rgb()).collect();
        rgb.sort();
        rgb.dedup();
        rgb
    };
    assert_eq!(unique.len(), inks.len(), "{inks:?}");
}

#[test]
fn an_update_available_install_wears_the_live_green() {
    assert_eq!(
        Ink::for_status(ProbeStatus::UpdateAvailable),
        Ink::for_status(ProbeStatus::Set)
    );
}

#[test]
fn an_applied_tuning_is_the_green_one() {
    let (red, green, blue) = Ink::for_status(ProbeStatus::Set).rgb();

    assert!(green > red && green > blue, "{red} {green} {blue}");
}

#[test]
fn the_icon_defaults_to_light_because_most_bars_are_dark() {
    assert_eq!(Ink::resting(None), Ink::Light);
}

#[test]
fn asking_for_a_dark_icon_gets_one() {
    assert_eq!(Ink::resting(Some("dark")), Ink::Dark);
    assert_eq!(Ink::resting(Some("  dark  ")), Ink::Dark);
}

#[test]
fn a_typo_leaves_a_visible_icon_rather_than_no_icon() {
    assert_eq!(Ink::resting(Some("drak")), Ink::Light);
    assert_eq!(Ink::resting(Some("")), Ink::Light);
}
