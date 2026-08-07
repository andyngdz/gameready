//! `gameready init`.

use std::path::Path;

use anyhow::{Context as _, Result};
use gameready_core::exec::CommandRunner;
use gameready_core::facts;
use gameready_core::facts::PackageManagerKind;
use gameready_core::improvement::CoreCx;
use gameready_core::infra::pkg;
use gameready_core::infra::steam::discover_setups;
use gameready_core::journal::{Journal, RunId, StatePaths};
use gameready_core::run::{Mode, RunPlan, RunReport, apply_plan, plan_run};
use gameready_core::steam::{GameSetup, Overlay};
use gameready_core::steps::core_steps;

use crate::cli::commands::constants::{CANNOT_OPEN_JOURNAL, CANNOT_READ_SYSTEM};
use crate::cli::ui::{self, Answers, Picker, Questions};

/// What the flags asked for, before any question is put to the user.
pub struct InitRequest<'a> {
    pub games_dir: &'a Path,
    pub mode: Mode,
    pub picker: Picker,
    /// `Some` only when a flag set it; otherwise the run asks.
    pub overlay: Option<Overlay>,
}

impl InitRequest<'_> {
    /// The questions this run has, with the flags that already answered some of
    /// them folded in.
    fn questions<'a>(
        &self,
        setups: &'a [GameSetup],
        plan: &'a RunPlan,
        packages: PackageManagerKind,
    ) -> Questions<'a> {
        Questions {
            setups,
            plan,
            packages,
            picker: self.picker,
            overlay: self.overlay,
            mode: self.mode,
        }
    }

    /// What the user agreed to, rendered before the run carries it out.
    ///
    /// A dry run is the one that lists the packages: a real run installs them a
    /// moment later and the summary reports what landed, but a dry run has only
    /// this screen to say what it would have done.
    fn agreed_plan(
        &self,
        setups: &[GameSetup],
        answers: &Answers,
        run_plan: &RunPlan,
        packages: PackageManagerKind,
    ) -> String {
        let mut plan = ui::InitPlan::new(setups, answers, self.mode).to_string();
        if !self.mode.mutates() {
            plan.push_str(&ui::InstallList::new(run_plan, packages).to_string());
        }
        plan
    }
}

/// Asks everything, then does everything.
///
/// The order is the contract. Every question the run has is answered before the
/// first change, including the password prompt, so a user is never asked to
/// decide something once the alternative has already been taken away from them.
pub fn run(
    runner: &dyn CommandRunner,
    paths: StatePaths,
    request: &InitRequest<'_>,
    authorize: &dyn Fn() -> Result<()>,
) -> Result<(RunReport, String)> {
    let facts = facts::probe(runner).context(CANNOT_READ_SYSTEM)?;
    let packages = pkg::for_kind(facts.distro.package_manager());
    let cx = CoreCx::new(&facts, runner).with_packages(packages.as_ref());

    // Probing and resolving only read, so the whole picture is available while
    // every answer the user could give is still available too.
    let setups = discover_setups(request.games_dir);
    let mut progress = ui::ProgressView::new();
    let run_plan = plan_run(core_steps(), &cx, &mut |event| progress.on_event(event));
    drop(progress);

    let family = facts.distro.package_manager();
    let answers = ui::ask_everything(&request.questions(&setups, &run_plan, family))?;
    let mut out = request.agreed_plan(&setups, &answers, &run_plan, family);

    // Nothing above this line changed anything. Nothing below it asks.
    authorize()?;

    let mut journal =
        Journal::open(paths.clone(), RunId::generate()).context(CANNOT_OPEN_JOURNAL)?;
    let mut progress = ui::ProgressView::new();
    let report = apply_plan(
        run_plan,
        &cx,
        &mut journal,
        request.mode,
        answers.consent,
        &mut |event| progress.on_event(event),
    )?;
    drop(progress);

    out.push_str(&ui::Summary::new(&report, &paths.journal()).to_string());
    out.push_str(
        &answers
            .launch
            .carry_out(runner, &facts, &mut journal, &answers)?,
    );
    Ok((report, out))
}

#[cfg(test)]
#[path = "init_test.rs"]
mod init_test;
