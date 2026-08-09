//! The memory-manager parameters that decide whether an allocation stalls.

/// One kernel parameter and the value gameready wants it to hold.
///
/// `key` is the sysctl name, and the file under `/proc/sys` is derived from it
/// by swapping the dots for slashes. `why` is one sentence for the plan screen,
/// because a table of five numbers with no explanation is a change nobody can
/// consent to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LatencyKnob {
    /// The sysctl name, as `sysctl -w` spells it.
    pub key: &'static str,

    /// The value to set.
    pub target: &'static str,

    /// What holding that value buys, in one line.
    ///
    /// Kept under about 65 characters for the same reason as
    /// [`crate::steps::domain::GamingTool::what`]: nothing wraps it.
    pub why: &'static str,
}

impl LatencyKnob {
    /// Where the kernel exposes this parameter's live value.
    ///
    /// `vm.page-lock-unfairness` is the reason this is a translation and not a
    /// concatenation: only the dots become slashes, and the hyphen stays.
    #[must_use]
    pub fn runtime_path(&self) -> std::path::PathBuf {
        std::path::Path::new("/proc/sys").join(self.key.replace('.', "/"))
    }
}

/// Every parameter `core.sysctl.vm-latency` sets.
///
/// The set is the one CachyOS and the gaming-tuned Fedora guides converge on.
/// None of them raise a ceiling or hand out more memory; each one trades a
/// small amount of background housekeeping for a shorter worst-case wait when a
/// game asks for a page. That is why the step is sold as fewer stutters rather
/// than more frames.
///
/// `vm.swappiness` is deliberately absent: `core.memory.swappiness` owns that
/// key, and two steps writing it would clobber each other's rollback.
pub const VM_LATENCY_KNOBS: [LatencyKnob; 5] = [
    LatencyKnob {
        key: "vm.compaction_proactiveness",
        target: "0",
        why: "stops background defragmentation pausing a running game",
    },
    LatencyKnob {
        key: "vm.page-lock-unfairness",
        target: "1",
        why: "cuts how long a thread waits on a contended page",
    },
    LatencyKnob {
        key: "vm.watermark_scale_factor",
        target: "500",
        why: "starts reclaiming earlier, so allocations wait less",
    },
    LatencyKnob {
        key: "vm.dirty_background_ratio",
        target: "3",
        why: "flushes writes sooner, in smaller batches",
    },
    LatencyKnob {
        key: "vm.dirty_ratio",
        target: "8",
        why: "caps how much writeback can pile up before a stall",
    },
];

#[cfg(test)]
#[path = "vm_latency_test.rs"]
mod vm_latency_test;
