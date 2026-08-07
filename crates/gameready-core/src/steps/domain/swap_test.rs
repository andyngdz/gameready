use indoc::indoc;

use super::*;

const DISK_ONLY: &str = indoc! {"
    Filename Type Size Used Priority
    /swap.img file 8388604 53956 -1
"};

const ZRAM_PRIMARY: &str = indoc! {"
    Filename Type Size Used Priority
    /dev/zram0 partition 8388604 0 100
    /swap.img file 8388604 53956 -1
"};

#[test]
fn a_disk_swap_file_is_not_zram() {
    let areas = parse_proc_swaps(DISK_ONLY);
    assert_eq!(areas.len(), 1);

    let swap_file = &areas[0];
    assert_eq!(swap_file.backing, SwapBacking::Disk);
    assert_eq!(swap_file.priority, -1);
    assert!(!primary_is_zram(&areas));
}

#[test]
fn zram_at_the_top_priority_is_the_primary() {
    let areas = parse_proc_swaps(ZRAM_PRIMARY);
    assert_eq!(areas.len(), 2);
    assert_eq!(areas[0].backing, SwapBacking::Zram);
    assert!(primary_is_zram(&areas));
}

#[test]
fn zram_below_a_disk_swap_is_not_the_primary() {
    // Priority decides, not order: a disk area at higher priority is filled
    // first, so tuning swappiness for zram would be wrong.
    let areas = parse_proc_swaps(indoc! {"
        Filename Type Size Used Priority
        /dev/zram0 partition 100 0 -2
        /swap.img file 100 0 -1
    "});
    assert!(!primary_is_zram(&areas));
}

#[test]
fn a_tie_at_the_top_breaks_toward_zram() {
    let areas = parse_proc_swaps(indoc! {"
        Filename Type Size Used Priority
        /dev/zram0 partition 100 0 5
        /swap.img file 100 0 5
    "});
    assert!(primary_is_zram(&areas));
}

#[test]
fn no_swap_at_all_is_not_zram() {
    assert!(!primary_is_zram(&parse_proc_swaps(
        "Filename Type Size Used Priority\n"
    )));
    assert!(!primary_is_zram(&parse_proc_swaps("")));
}

#[test]
fn a_short_line_is_skipped_not_fatal() {
    let areas = parse_proc_swaps(indoc! {"
        Filename Type Size Used Priority
        /dev/zram0 partition 100
        /swap.img file 100 0 -1
    "});
    assert_eq!(areas.len(), 1);
    assert_eq!(areas[0].backing, SwapBacking::Disk);
}
