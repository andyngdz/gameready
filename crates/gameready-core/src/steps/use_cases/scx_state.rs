//! Reading what the kernel is scheduling with.

use std::path::Path;

use crate::exec::CommandRunner;
use crate::steps::constants::{SCHED_EXT_DISABLED, SCHED_EXT_OPS, SCHED_EXT_STATE};
use crate::steps::domain::SchedExt;

/// Reads the kernel's scheduler state.
///
/// Returns an answer rather than a `Result`: a state file that cannot be read
/// is not a fault to report, it is the kernel saying it has no sched_ext, and
/// every caller would have to turn that error into the same answer anyway.
pub fn read_sched_ext(runner: &dyn CommandRunner) -> SchedExt {
    let Ok(state) = runner.read_to_string(Path::new(SCHED_EXT_STATE)) else {
        return SchedExt::Unsupported;
    };

    if state.trim() == SCHED_EXT_DISABLED {
        return SchedExt::Idle;
    }

    SchedExt::Running {
        scheduler: runner
            .read_to_string(Path::new(SCHED_EXT_OPS))
            .ok()
            .map(|ops| ops.trim().to_owned())
            .filter(|ops| !ops.is_empty()),
    }
}

#[cfg(test)]
#[path = "scx_state_test.rs"]
mod scx_state_test;
