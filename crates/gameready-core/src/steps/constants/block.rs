//! Block devices and the udev rule that pins their I/O scheduler.

/// The directory the kernel lists every whole block device under.
pub const SYS_BLOCK: &str = "/sys/block";

/// A block device's scheduler attribute, relative to its `/sys/block` entry.
pub const QUEUE_SCHEDULER: &str = "queue/scheduler";

/// A block device's rotational flag, relative to its `/sys/block` entry.
pub const QUEUE_ROTATIONAL: &str = "queue/rotational";

/// The udev rule that makes the scheduler choice survive a reboot.
///
/// Its own file, never an edit of an existing one, so losing the journal still
/// leaves a change that is identifiable and removable.
pub const IO_SCHEDULER_RULE: &str = "/etc/udev/rules.d/60-gameready-ioscheduler.rules";
