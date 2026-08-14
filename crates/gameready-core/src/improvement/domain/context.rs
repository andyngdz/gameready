//! What a step is handed to do its work.

use crate::exec::CommandRunner;
use crate::facts::SystemFacts;
use crate::improvement::errors::StepError;
use crate::journal::{Change, Journal, JournalEvent, RunId};
use crate::pkg::PackageManager;

use super::doing::Doing;
use super::identity::ImprovementId;

/// Read-only context for a system-wide step.
///
/// Carries no way to mutate anything. `probe`, `plan`, and `verify` take this,
/// which is what makes "probing must not change the system" a property the type
/// system holds rather than a rule reviewers have to remember.
#[derive(Clone, Copy)]
pub struct CoreCx<'a> {
    /// What was probed about this machine before the run started.
    pub facts: &'a SystemFacts,

    /// The only route to the system. Reads are free; mutations belong in
    /// `apply` and must go through [`ApplyCx::mutate`].
    pub runner: &'a dyn CommandRunner,

    /// The distro's package tooling, for steps that install something of their
    /// own rather than declaring it through `dependencies()`.
    ///
    /// Optional because a step that touches no packages must still be testable
    /// without standing one up, and because the doctor and rollback paths build
    /// a context without ever installing anything. A step that needs one and
    /// finds `None` reports that it cannot tell rather than assuming.
    pub packages: Option<&'a dyn PackageManager>,

    /// Whether the CPU governor step should make its change survive a reboot.
    ///
    /// The default is `false`: the governor is written live and lasts until the
    /// next boot. The user answers a fifth question to turn this on, and only
    /// when the run would actually pin the governor. Set on the context rather
    /// than passed to `apply` so the run machinery stays the same for every
    /// step.
    pub governor_pinned: bool,
}

impl<'a> CoreCx<'a> {
    /// Builds a context from probed facts and a runner, with no package
    /// tooling. The common case: most steps write files and set kernel
    /// parameters.
    #[must_use]
    pub const fn new(facts: &'a SystemFacts, runner: &'a dyn CommandRunner) -> Self {
        Self {
            facts,
            runner,
            packages: None,
            governor_pinned: false,
        }
    }

    /// Adds the package tooling a step needs to install something itself.
    #[must_use]
    pub const fn with_packages(mut self, packages: &'a dyn PackageManager) -> Self {
        self.packages = Some(packages);
        self
    }

    /// Sets whether the CPU governor change should persist across reboots.
    #[must_use]
    pub const fn with_governor_pinned(mut self, pinned: bool) -> Self {
        self.governor_pinned = pinned;
        self
    }
}

type ProgressCallback<'a> = std::cell::RefCell<Box<dyn FnMut(Doing) + 'a>>;

/// Mutating context, handed only to `apply` and `rollback`.
///
/// The single way to change the system is [`ApplyCx::mutate`], which makes the
/// undo record durable before the change happens. A step that reaches for the
/// runner to mutate has bypassed that and is a bug; the clippy
/// `disallowed_methods` entry in `clippy.toml` catches the common shape.
///
/// Generic over the read-only context so the same apply machinery serves both
/// system-wide steps and per-game steps, which need different read-only state.
pub struct ApplyCx<'a, C> {
    /// The read-only context, still available for probing mid-apply.
    pub cx: C,
    step: ImprovementId,
    runner: &'a dyn CommandRunner,
    journal: &'a mut Journal,
    recorded: Vec<Change>,
    on_progress: Option<ProgressCallback<'a>>,
}

impl<'a, C> ApplyCx<'a, C> {
    /// Wraps a read-only context with the runner and journal for one step.
    pub fn new(
        cx: C,
        step: ImprovementId,
        runner: &'a dyn CommandRunner,
        journal: &'a mut Journal,
    ) -> Self {
        Self {
            cx,
            step,
            runner,
            journal,
            recorded: Vec::new(),
            on_progress: None,
        }
    }

    /// Which step is applying.
    #[must_use]
    pub const fn step(&self) -> &ImprovementId {
        &self.step
    }

    /// Which run this belongs to.
    ///
    /// Steps stamp it into every file they create, so `doctor` can map a
    /// leftover file back to the run that made it even when the journal is
    /// gone.
    #[must_use]
    pub const fn run(&self) -> RunId {
        self.journal.run()
    }

    /// Where this run keeps pre-images of files it replaces.
    ///
    /// A step that edits a file it did not create needs somewhere to put the
    /// original, and the directory is per run so two runs never overwrite each
    /// other's copy.
    #[must_use]
    pub fn backup_dir(&self) -> std::path::PathBuf {
        self.journal.paths().backups(self.journal.run())
    }

    /// Reads through the runner without recording anything.
    ///
    /// For a step that needs to look at the system mid-apply, such as reading a
    /// value back before overwriting it. Mutating through this bypasses the
    /// journal and is what `disallowed_methods` is configured to catch.
    #[must_use]
    pub const fn reader(&self) -> &'a dyn CommandRunner {
        self.runner
    }

    /// Attaches a progress callback so steps can report sub-phases.
    ///
    /// The CLI renders these as a checklist with spinners, so a step that
    /// downloads, verifies, and extracts shows each phase individually
    /// rather than one long-running spinner.
    #[must_use]
    pub fn with_progress(mut self, callback: Box<dyn FnMut(Doing) + 'a>) -> Self {
        self.on_progress = Some(std::cell::RefCell::new(callback));
        self
    }

    /// Reports a sub-phase to the CLI.
    pub fn progress(&self, message: &str) {
        self.report(Doing::Phase(message.to_owned()));
    }

    /// Reports how much of a known total has landed.
    ///
    /// For a step that knows what it is fetching before it starts. A total of
    /// zero is not reported at all: a bar with no end is worse than the spinner
    /// it would replace.
    pub fn bytes(&self, done: u64, total: u64) {
        if total > 0 {
            self.report(Doing::Bytes { done, total });
        }
    }

    /// Hands one report to whoever is listening.
    ///
    /// Does nothing when no callback is attached, so steps that report are
    /// harmless in tests and dry runs.
    fn report(&self, doing: Doing) {
        let Some(ref cell) = self.on_progress else {
            return;
        };
        if let Ok(mut callback) = cell.try_borrow_mut() {
            callback(doing);
        }
    }

    /// Performs one mutation, with its undo record on disk first.
    ///
    /// The ordering is the safety property of the whole design: `change` is
    /// appended to the journal and fsync'd, and only then does `mutate_fn` run.
    /// An interrupt at any point therefore leaves the system in a state that is
    /// a prefix of a fully undoable sequence, never ahead of one.
    ///
    /// If journalling fails, `mutate_fn` never runs and nothing is changed.
    pub fn mutate<T, F>(&mut self, change: Change, mutate_fn: F) -> Result<T, StepError>
    where
        F: FnOnce(&dyn CommandRunner) -> Result<T, StepError>,
    {
        self.journal.append(JournalEvent::Changed {
            step: self.step.clone(),
            change: change.clone(),
        })?;

        // Recorded before the mutation runs, not after. A command that fails
        // partway may still have changed something, and the undo record is the
        // only way to find out and put it back.
        self.recorded.push(change);

        mutate_fn(self.runner)
    }

    /// Records a change without running a mutation against the system.
    ///
    /// For the rare change whose mutation is clearing the way for a later one,
    /// such as stopping a unit a takeover then re-points. Written and fsync'd
    /// before the caller's own next mutation, like every other record, so an
    /// interrupt between the two still leaves a fully undoable prefix.
    pub fn record(&mut self, change: Change) -> Result<(), StepError> {
        self.journal.append(JournalEvent::Changed {
            step: self.step.clone(),
            change: change.clone(),
        })?;
        self.recorded.push(change);
        Ok(())
    }

    /// Every change recorded during this step, in the order performed.
    ///
    /// The executor hands these to `rollback` when verification fails, so a
    /// step undoes exactly what it did rather than what it meant to do.
    #[must_use]
    pub fn recorded(&self) -> &[Change] {
        &self.recorded
    }
}

#[cfg(test)]
#[path = "context_test.rs"]
mod context_test;
