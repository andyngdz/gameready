use super::*;

#[test]
fn no_two_statuses_are_drawn_in_the_same_colour() {
    let inks = [
        Ink::for_status(ProbeStatus::Set),
        Ink::for_status(ProbeStatus::Ready),
        Ink::for_status(ProbeStatus::Attention),
        Ink::for_status(ProbeStatus::Inactive),
    ];

    for (index, first) in inks.iter().enumerate() {
        for second in &inks[index + 1..] {
            assert_ne!(first.rgb(), second.rgb(), "{inks:?}");
        }
    }
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
