use super::*;

#[test]
fn the_two_answers_read_differently() {
    // The safe default and the persistent choice must never render the same, or
    // the user cannot tell which one they are picking.
    assert_ne!(
        Persistence::ThisBoot.to_string(),
        Persistence::KeepIt.to_string()
    );
}

#[test]
fn this_boot_names_that_a_reboot_undoes_it() {
    assert!(Persistence::ThisBoot.to_string().contains("reboot"));
}
