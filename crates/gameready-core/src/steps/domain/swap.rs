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

impl SwapBacking {
    /// The word the doctor screen prints for this backing.
    #[must_use]
    pub const fn describe(self) -> &'static str {
        match self {
            Self::Zram => "zram",
            Self::Disk => "disk",
        }
    }
}

/// One active swap area: what backs it, its size, and the priority the kernel
/// gives it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwapArea {
    /// What storage the area lives on.
    pub backing: SwapBacking,
    /// The kernel swaps to the highest priority first.
    pub priority: i32,
    /// Size in kibibytes, the unit `/proc/swaps` reports.
    pub size_kib: u64,
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
    let [filename, _type, size, _used, priority, ..] = columns.as_slice() else {
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
        size_kib: size.parse().ok()?,
    })
}

/// The swap the kernel fills first, and the total size across every area.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActiveSwap {
    /// The backing of the primary area, the one tuning is decided by.
    pub backing: SwapBacking,
    /// Total size of every active area, in kibibytes.
    pub total_kib: u64,
}

/// Summarises the active swap for the doctor screen: the primary area's
/// backing and the total size, or `None` when the machine has no swap.
#[must_use]
pub fn active_swap(areas: &[SwapArea]) -> Option<ActiveSwap> {
    if areas.is_empty() {
        return None;
    }
    let backing = if primary_is_zram(areas) {
        SwapBacking::Zram
    } else {
        SwapBacking::Disk
    };
    Some(ActiveSwap {
        backing,
        total_kib: areas.iter().map(|area| area.size_kib).sum(),
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
