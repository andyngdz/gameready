//! Explain why gameready does not pin the CPU governor.

use std::path::Path;

use crate::improvement::{
    ApplyCx, CoreCx, CoreImprovement, Improvement, ImprovementId, Privilege, Probe, StepError,
    StepPlan, Tag, Verification,
};
use crate::journal::Change;
use crate::steps::constants::SCALING_GOVERNOR;

/// A step that always declines, and says why.
///
/// It ships as a step rather than as a line in the documentation because
/// "why does gameready not set the governor to performance" is the question
/// this project will be asked most, and the answer belongs where the user is
/// already looking: on the run summary, next to the steps that did act.
#[derive(Debug, Default, Clone, Copy)]
pub struct CpuGovernor;

impl CpuGovernor {
    /// The step's stable id.
    #[must_use]
    pub const fn id_const() -> ImprovementId {
        ImprovementId::from_static("core.cpu.governor")
    }

    /// What the first core's governor is set to right now, when it can be read.
    ///
    /// A machine with no `cpufreq` at all, which is normal in a virtual
    /// machine, simply has nothing to report here.
    fn current(cx: &CoreCx<'_>) -> Option<String> {
        cx.runner
            .read_to_string(Path::new(SCALING_GOVERNOR))
            .ok()
            .map(|raw| raw.trim().to_owned())
            .filter(|governor| !governor.is_empty())
    }
}

impl Improvement for CpuGovernor {
    fn id(&self) -> ImprovementId {
        Self::id_const()
    }

    fn name(&self) -> &str {
        "Leave the CPU governor to gamemode"
    }

    fn rationale(&self) -> &str {
        "Pinning the governor to performance system-wide keeps the CPU at high \
         clocks all day, which on a laptop costs battery and thermal headroom \
         and gains nothing outside a game. gamemode already raises the governor \
         when a game starts and lowers it when the game exits, which is the \
         same benefit for the time it actually matters. Installing gamemode is \
         what core.pkg.tools does."
    }

    fn privilege(&self) -> Privilege {
        Privilege::User
    }

    fn tags(&self) -> &[Tag] {
        &[Tag::Cpu]
    }
}

impl CoreImprovement for CpuGovernor {
    fn probe(&self, cx: &CoreCx<'_>) -> Result<Probe, StepError> {
        let governor = Self::current(cx).map_or_else(
            || "this machine reports no CPU governor".to_owned(),
            |governor| format!("the governor is `{governor}`"),
        );

        Ok(Probe::NotApplicable {
            reason: format!("{governor}, gamemode handles this per-game"),
        })
    }

    // The three below cannot run: `probe` never returns `Applicable`, so the
    // executor never reaches them. They fail loudly rather than quietly doing
    // nothing, so a future change to the executor surfaces here instead of
    // silently turning a declining step into a mutating one.
    fn plan(&self, _cx: &CoreCx<'_>) -> Result<StepPlan, StepError> {
        Ok(StepPlan::new(self.id(), "no change, by design"))
    }

    fn apply(&self, _cx: &mut ApplyCx<'_, CoreCx<'_>>) -> Result<(), StepError> {
        Err(StepError::PreconditionLost {
            step: self.id(),
            detail: "this step never applies; gamemode owns the governor".to_owned(),
        })
    }

    fn verify(&self, _cx: &CoreCx<'_>) -> Result<Verification, StepError> {
        // Empty on purpose. Verification proves a change took effect, and this
        // step makes none, so a passing check here would be inventing evidence.
        Ok(Verification::new())
    }

    fn rollback(
        &self,
        _undo: &[Change],
        _cx: &mut ApplyCx<'_, CoreCx<'_>>,
    ) -> Result<(), StepError> {
        Ok(())
    }
}

#[cfg(test)]
#[path = "cpu_governor_test.rs"]
mod cpu_governor_test;
