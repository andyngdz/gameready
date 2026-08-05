use std::path::PathBuf;

use tempfile::TempDir;

use super::*;
use crate::facts::SystemFacts;
use crate::improvement::KernelVersion;
use crate::infra::exec::MockRunner;
use crate::journal::{Journal, RunId, StatePaths};

fn facts() -> SystemFacts {
    SystemFacts::new(KernelVersion::new(7, 0, 0), "7.0.0-29-generic".to_owned())
}

fn change() -> Change {
    Change::DirCreated {
        path: PathBuf::from("/tmp/gameready-test"),
    }
}

#[test]
fn mutate_makes_the_undo_record_durable_before_the_mutation_runs() {
    let state = TempDir::new().expect("temp dir");
    let run = RunId::generate();
    let state_paths = StatePaths::new(state.path().to_path_buf());
    let runner = MockRunner::new();
    let facts = facts();
    let step = ImprovementId::from_static("test.step");

    let journal_path = {
        let mut journal = Journal::open(state_paths.clone(), run).expect("journal opens");
        let path = state_paths.journal();
        let mut cx = ApplyCx::new(CoreCx::new(&facts, &runner), step, &runner, &mut journal);

        // Read the journal from inside the mutation. If the record is not
        // already on disk at this point, a crash here would leave a change with
        // no way back, which is the failure the whole design exists to prevent.
        let visible_during_mutation = cx
            .mutate(change(), |_| {
                Ok(crate::journal::load(&path).expect("journal readable mid-mutation"))
            })
            .expect("mutation runs");

        assert_eq!(
            visible_during_mutation.len(),
            1,
            "the undo record was not durable before the mutation ran"
        );
        path
    };

    let after = crate::journal::load(&journal_path).expect("reads");
    assert_eq!(after.len(), 1);
}

#[test]
fn a_failed_mutation_is_still_recorded_for_rollback() {
    let state = TempDir::new().expect("temp dir");
    let run = RunId::generate();
    let mut journal =
        Journal::open(StatePaths::new(state.path().to_path_buf()), run).expect("journal opens");
    let runner = MockRunner::new();
    let facts = facts();
    let step = ImprovementId::from_static("test.step");

    let mut cx = ApplyCx::new(CoreCx::new(&facts, &runner), step, &runner, &mut journal);
    let outcome: Result<(), StepError> = cx.mutate(change(), |_| {
        Err(StepError::PreconditionLost {
            step: ImprovementId::from_static("test.step"),
            detail: "gone".to_owned(),
        })
    });

    assert!(outcome.is_err());
    // The command may have changed something before failing, so the undo
    // record has to survive the failure.
    assert_eq!(cx.recorded(), &[change()]);
}
