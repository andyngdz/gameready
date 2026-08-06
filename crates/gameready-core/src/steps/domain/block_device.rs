//! Classifying block devices and reading their I/O scheduler.

/// Names in `/sys/block` that are not real disks worth tuning: memory-backed
/// devices, device-mapper targets, optical drives, software RAID, and network
/// block devices.
const VIRTUAL_PREFIXES: [&str; 8] = ["loop", "ram", "zram", "dm-", "md", "sr", "nbd", "fd"];

/// The scheduler for an NVMe drive.
///
/// None at all: the drive queues deeply in hardware, so a software scheduler
/// only adds latency.
pub const NVME_SCHEDULER: &str = "none";

/// The scheduler for a spinning disk.
///
/// bfq, whose fairness keeps the desktop responsive while a game streams from
/// a slow disk.
pub const ROTATIONAL_SCHEDULER: &str = "bfq";

/// The scheduler for a SATA or SAS solid-state disk.
///
/// mq-deadline, a light scheduler that still bounds worst-case latency.
pub const SSD_SCHEDULER: &str = "mq-deadline";

/// Whether a `/sys/block` entry names a real disk worth tuning.
///
/// Conservative in the opposite direction from the game scan: a virtual device
/// that slips through gets a harmless scheduler write, so the prefix list can
/// stay short without risking a real disk being skipped.
#[must_use]
pub fn is_tunable_disk(name: &str) -> bool {
    !name.is_empty()
        && !VIRTUAL_PREFIXES
            .iter()
            .any(|prefix| name.starts_with(prefix))
}

/// One block device and the single fact that picks its scheduler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockDevice {
    /// The kernel name, such as `nvme0n1` or `sda`.
    pub name: String,
    /// Whether the kernel reports the device as a spinning disk.
    pub rotational: bool,
}

impl BlockDevice {
    /// The scheduler gameready sets for this device.
    ///
    /// NVMe wins over the rotational flag: an NVMe drive always reports
    /// non-rotational, and the name is the reliable signal for the deep-queue
    /// case that wants no scheduler.
    #[must_use]
    pub fn target_scheduler(&self) -> &'static str {
        if self.name.starts_with("nvme") {
            NVME_SCHEDULER
        } else if self.rotational {
            ROTATIONAL_SCHEDULER
        } else {
            SSD_SCHEDULER
        }
    }
}

/// A device's active scheduler and the full set it offers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchedulerState {
    /// The scheduler currently in effect.
    pub active: String,
    /// Every scheduler the device can be switched to, the active one included.
    pub available: Vec<String>,
}

impl SchedulerState {
    /// Whether the device can be switched to `scheduler`.
    ///
    /// A target the kernel does not offer, such as bfq when its module is not
    /// loaded, cannot be set; the step leaves such a device alone rather than
    /// writing a value the write would reject.
    #[must_use]
    pub fn offers(&self, scheduler: &str) -> bool {
        self.available.iter().any(|name| name == scheduler)
    }
}

/// Reads a `queue/scheduler` line into the active scheduler and the options.
///
/// The line reads like `none [mq-deadline] kyber bfq`: every token is an
/// option and the one in brackets is active. Returns `None` when no token is
/// bracketed, which is not a scheduler line gameready understands.
#[must_use]
pub fn parse_scheduler_line(line: &str) -> Option<SchedulerState> {
    let mut active = None;
    let mut available = Vec::new();
    for token in line.split_whitespace() {
        let name = token.trim_start_matches('[').trim_end_matches(']');
        if name.is_empty() {
            continue;
        }
        if token.starts_with('[') {
            active = Some(name.to_owned());
        }
        available.push(name.to_owned());
    }
    active.map(|active| SchedulerState { active, available })
}

#[cfg(test)]
#[path = "block_device_test.rs"]
mod block_device_test;
