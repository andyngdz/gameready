//! `gameready rollback`.

use anyhow::Result;
use gameready_core::journal::{self, StatePaths};

/// Reports what a rollback would undo.
///
/// Replaying the undo records is M2 work. Until then this reads the journal and
/// says what it holds, rather than claiming to have undone anything.
pub fn run(paths: &StatePaths, run: Option<&str>) -> Result<String> {
    let records = journal::load(&paths.journal())?;
    let selected = run.unwrap_or("the most recent run");

    Ok(format!(
        "\nRollback is not implemented yet (M2).\n\n  \
         Journal   {}\n  Records   {}\n  Target    {selected}\n\n\
         Nothing was changed.\n",
        paths.journal().display(),
        records.len(),
    ))
}

#[cfg(test)]
#[path = "rollback_test.rs"]
mod rollback_test;
