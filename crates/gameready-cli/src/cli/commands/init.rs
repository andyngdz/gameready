//! `gameready init`.

use std::path::Path;

use anyhow::{Context as _, Result};
use gameready_core::exec::CommandRunner;
use gameready_core::facts;
use gameready_core::facts::PackageManagerKind;
use gameready_core::improvement::CoreCx;
use gameready_core::infra::pkg;
use gameready_core::infra::steam::{discover_setups, installed_compat_tools, locate_steam_dir};
use gameready_core::journal::{Journal, RunId, StatePaths};
use gameready_core::run::{apply_plan, plan_run, Mode, RunPlan, RunReport};
use gameready_core::steam::{GameSetup, Overlay};
use gameready_core::steps::core_steps;

use crate::cli::commands::constants::{CANNOT_OPEN_JOURNAL, CANNOT_READ_SYSTEM};
use crate::cli::escalation::Escalation;
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
        compat_tools: &'a [String],
    ) -> Questions<'a> {
        Questions {
            setups,
            plan,
            packages,
            picker: self.picker,
            overlay: self.overlay,
            mode: self.mode,
            compat_tools,
        }
    }

    /// Every question this run has, asked once, with the agreed plan rendered.
    fn ask(
        &self,
        setups: &[GameSetup],
        run_plan: &RunPlan,
        packages: PackageManagerKind,
        compat_tools: &[String],
    ) -> Result<(Answers, String)> {
        let answers =
            ui::ask_everything(&self.questions(setups, run_plan, packages, compat_tools))?;
        let rendered = self.agreed_plan(setups, &answers, run_plan, packages);
        Ok((answers, rendered))
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
        let mut plan =
            ui::InitPlan::new(setups, answers, run_plan, packages, self.mode).to_string();
        if !self.mode.mutates() {
            plan.push_str(&ui::InstallList::new(run_plan, packages).to_string());
        }
        plan
    }

    /// Puts the agreed plan where the user will read it before deciding.
    ///
    /// A run that is about to change something shows it on stderr right before
    /// the password prompt, which is the last moment it can still be stopped.
    /// A dry run has nothing to stop, so its plan is the report itself and goes
    /// to stdout with the rest.
    fn checkpoint(&self, rendered: String) -> String {
        if !self.mode.mutates() {
            return rendered;
        }
        if console::user_attended_stderr() {
            eprint!("{rendered}");
        }
        String::new()
    }
}

/// The compatibility tools this machine has, by directory name.
///
/// Read before the first question, because a profile asking for the newest
/// GE-Proton can only resolve against the builds that are there now. A machine
/// with no Steam has none, which is not a failure worth reporting here.
fn compat_tools(steam: Option<&Path>) -> Vec<String> {
    steam.map(installed_compat_tools).unwrap_or_default()
}

/// What the games line on the opening screen reports.
///
/// The count comes from the paired setups rather than from the Steam directory
/// itself: a run can only tune a game it found installed, so that is the number
/// the user is about to be asked about.
fn games_found(steam: Option<&Path>, setups: usize) -> ui::SteamGames {
    if steam.is_some() {
        ui::SteamGames::Found(setups)
    } else {
        ui::SteamGames::Missing
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
    escalation: Escalation<'_>,
) -> Result<(RunReport, String)> {
    let facts = facts::probe(runner).context(CANNOT_READ_SYSTEM)?;
    let packages = pkg::for_kind(facts.distro.package_manager());
    let cx = CoreCx::new(&facts, runner).with_packages(packages.as_ref());

    // Probing and resolving only read, so the whole picture is available while
    // every answer the user could give is still available too.
    let steam = locate_steam_dir().ok();
    let setups = discover_setups(request.games_dir);
    ui::LookingAtMachine::show(&facts, games_found(steam.as_deref(), setups.len()));

    let mut progress = ui::ProgressView::new();
    let run_plan = plan_run(core_steps(), &cx, &mut |event| progress.on_event(event));
    drop(progress);

    let family = facts.distro.package_manager();
    let tools = compat_tools(steam.as_deref());
    let (answers, rendered) = request.ask(&setups, &run_plan, family, &tools)?;
    let mut out = request.checkpoint(rendered);

    // Nothing above this line changed anything. Nothing below it asks.
    escalation.ask()?;

    // The governor answer is known only now, so the context the run applies
    // with is rebuilt to carry it. Copy, so this shadows without disturbing the
    // borrows above.
    let cx = cx.with_governor_pinned(answers.governor_pinned);

    let mut journal =
        Journal::open(paths.clone(), RunId::generate()).context(CANNOT_OPEN_JOURNAL)?;
    let mut progress = ui::ProgressView::sweeping(request.mode, run_plan.to_apply());
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
