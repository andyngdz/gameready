//! Reading `/proc/swaps` to tell whether zram is the swap the kernel uses first.

/// What backs one active swap area.
///
/// Only the distinction that changes the swappiness decision is modelled: zram
/// compresses pages into RAM, so swapping aggressively is a win; anything on
/// disk pays a seek per swapped page, so the kernel default is right.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapBacking {
    /// A compressed RAM block device, `/dev/zram*`.
    Zram,
    /// A partition, file, or any other on-disk area.
    Disk,
}

/// One active swap area: what backs it and the priority the kernel gives it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwapArea {
    /// What storage the area lives on.
    pub backing: SwapBacking,
    /// The kernel swaps to the highest priority first.
    pub priority: i32,
}

/// Parses `/proc/swaps` into its active areas.
///
/// The first line is a header; every later line is
/// `Filename Type Size Used Priority`, whitespace-separated. A line missing any
/// of the five columns is skipped rather than failing the read, so a kernel
/// that adds a column cannot break the probe.
#[must_use]
pub fn parse_proc_swaps(contents: &str) -> Vec<SwapArea> {
    contents
        .lines()
        .skip(1)
        .filter_map(parse_swap_line)
        .collect()
}

/// Reads one `/proc/swaps` body line, or `None` when it is not a swap row.
///
/// The kernel's columns are `Filename Type Size Used Priority`. Only the first
/// and last decide anything here; the row is taken only when all five are
/// present, and any extra trailing column is ignored.
fn parse_swap_line(line: &str) -> Option<SwapArea> {
    let columns: Vec<&str> = line.split_whitespace().collect();
    let [filename, _type, _size, _used, priority, ..] = columns.as_slice() else {
        return None;
    };
    let backing = if filename.starts_with("/dev/zram") {
        SwapBacking::Zram
    } else {
        SwapBacking::Disk
    };
    Some(SwapArea {
        backing,
        priority: priority.parse().ok()?,
    })
}

/// Whether zram is the swap the kernel reaches for first.
///
/// "Primary" means highest priority, since that is the area the kernel fills
/// first. A tie breaks toward zram: if a zram area shares the top priority it
/// still carries the load worth tuning for. No swap at all returns false, which
/// sends the step to `NotApplicable`.
#[must_use]
pub fn primary_is_zram(areas: &[SwapArea]) -> bool {
    let Some(top) = areas.iter().map(|area| area.priority).max() else {
        return false;
    };
    areas
        .iter()
        .filter(|area| area.priority == top)
        .any(|area| area.backing == SwapBacking::Zram)
}

#[cfg(test)]
#[path = "swap_test.rs"]
mod swap_test;
