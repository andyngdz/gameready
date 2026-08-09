//! Which of the latency parameters this kernel actually has, and what they hold.

use crate::exec::CommandRunner;
use crate::improvement::StepError;
use crate::steps::domain::{LatencyKnob, VM_LATENCY_KNOBS};

/// One parameter this kernel exposes, and the value it currently holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnobReading {
    /// The parameter, carried whole so callers keep its target and its blurb.
    pub knob: LatencyKnob,

    /// What `/proc/sys` reads right now, trimmed.
    pub current: String,
}

impl KnobReading {
    /// Whether the kernel already holds the value gameready wants.
    #[must_use]
    pub fn already_set(&self) -> bool {
        self.current == self.knob.target
    }
}

/// Reads every parameter this kernel exposes, skipping the ones it does not.
///
/// A missing file is a skip rather than an error: the exact set of memory
/// manager knobs moves between kernel versions, and one absent parameter is no
/// reason to refuse the other four. A parameter that exists but cannot be read
/// is a real failure, because a value that cannot be read cannot be restored.
pub fn survey(runner: &dyn CommandRunner) -> Result<Vec<KnobReading>, StepError> {
    let mut readings = Vec::new();

    for knob in VM_LATENCY_KNOBS {
        let path = knob.runtime_path();
        if !runner.path_exists(&path) {
            continue;
        }

        let raw = runner.read_to_string(&path).map_err(StepError::Exec)?;
        readings.push(KnobReading {
            knob,
            current: raw.trim().to_owned(),
        });
    }

    Ok(readings)
}

#[cfg(test)]
#[path = "sysctl_vm_latency_survey_test.rs"]
mod sysctl_vm_latency_survey_test;
