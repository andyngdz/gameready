use indoc::indoc;

use crate::infra::exec::MockRunner;

use super::*;

const ZRAM_SWAPS: &str = indoc! {"
    Filename				Type		Size		Used		Priority
    /dev/zram0                              partition	8388604		0		100
"};

#[test]
fn a_machine_with_no_proc_swaps_reports_no_swap_rather_than_guessing() {
    // An unreadable swap layout must not become a guess at swappiness.
    assert!(read_swaps(&MockRunner::new()).is_empty());
}

#[test]
fn the_swap_areas_come_back_parsed() {
    let runner = MockRunner::new().with_file(PROC_SWAPS, ZRAM_SWAPS);

    assert_eq!(read_swaps(&runner).len(), 1);
}

#[test]
fn the_live_value_is_read_from_the_path_the_kernel_exposes() {
    let runner = MockRunner::new().with_file(runtime_path(), "60\n");

    assert_eq!(read_current(&runner).ok(), Some(60));
}

#[test]
fn a_value_that_is_not_a_number_is_a_parse_error_not_a_default() {
    let runner = MockRunner::new().with_file(runtime_path(), "not a number\n");

    assert!(matches!(
        read_current(&runner),
        Err(StepError::Parse { .. })
    ));
}

#[test]
fn a_missing_file_is_an_exec_error_rather_than_a_parse_one() {
    assert!(matches!(
        read_current(&MockRunner::new()),
        Err(StepError::Exec(_))
    ));
}
