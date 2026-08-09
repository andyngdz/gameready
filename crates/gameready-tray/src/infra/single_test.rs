use super::*;

#[test]
fn the_name_is_reverse_dns_so_it_cannot_collide_with_another_project() {
    // A bare name like "gameready" is not a valid well-known bus name and the
    // request would fail at run time, where nothing here would catch it.
    assert!(NAME.contains('.'), "{NAME}");
    assert!(NAME.starts_with("io.github."), "{NAME}");
    assert!(!NAME.ends_with('.'), "{NAME}");
}

#[test]
fn claiming_twice_in_one_process_still_reports_a_taken_name() {
    // Needs a session bus. Under `dbus-run-session` this is the real path a
    // second launch takes; without one there is nothing to be the only tray on,
    // so the test has nothing to say.
    let Ok(first) = claim() else {
        return;
    };
    let Claim::Ours(held) = first else {
        return;
    };

    let second = claim().expect("a second request on a working bus should answer");

    assert!(matches!(second, Claim::Taken));
    drop(held);
}
