//! Recording what a Steam config block held, and putting it back.
//!
//! Separate from the write side because it answers a different question. The
//! write side asks what a file should say; this asks what it said first, so an
//! undo can put back the keys the run touched without disturbing the ones it
//! did not.

use keyvalues_parser::{parse, Vdf};
use serde::{Deserialize, Serialize};

use crate::steam::errors::VdfError;
use crate::steam::service::vdf::{descend, read, write, Missing};

/// One key a run overwrote, and what it held first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PriorScalar {
    pub key: String,

    /// `None` when the run added the key, so undoing removes it again rather
    /// than leaving an empty string Steam would read as a real setting.
    pub value: Option<String>,
}

/// What a block held before a run wrote into it.
///
/// Recorded instead of a pre-image of the whole file. Steam rewrites
/// `localconfig.vdf` and `config.vdf` every time it exits, so putting a whole
/// file back would undo the run and discard everything the user changed in
/// Steam since, from newly installed games to controller bindings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "prior", rename_all = "snake_case")]
pub enum PriorBlock {
    /// The block was not in the file, so undoing removes the one the run added.
    Absent,

    /// The block was there. Only the keys the run wrote are listed; the rest of
    /// the block was never ours to put back.
    Present { entries: Vec<PriorScalar> },
}

/// One block a run wrote into, and what it held first.
///
/// A run sets launch options for several games in one write, so the undo has to
/// carry several of these and put them all back in the same write.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PriorSection {
    /// Blocks to descend, outermost first, including the document's own key.
    pub section: Vec<String>,
    pub prior: PriorBlock,
}

/// Puts every recorded block back, in one pass over the file.
///
/// One pass for the same reason the write side makes one: a file written once
/// per game would be left half undone by an interrupt between two of them.
pub fn restore_sections(text: &str, sections: &[PriorSection]) -> Result<String, VdfError> {
    let mut current = text.to_owned();
    for section in sections {
        let borrowed: Vec<&str> = section.section.iter().map(String::as_str).collect();
        current = restore_block(&current, &borrowed, &section.prior)?;
    }
    Ok(current)
}

/// Records what `keys` hold under `path`, for a caller about to overwrite them.
///
/// Taken before the write, so the undo names values that were really there
/// rather than values guessed afterwards from a file Steam may have rewritten.
pub fn capture_block(text: &str, path: &[&str], keys: &[&str]) -> Result<PriorBlock, VdfError> {
    let mut document: Vdf<'static> = Vdf::from(parse(text)?).into_owned();
    let rest = descend_path(&document, path)?;

    // A block the file does not have yet is the normal case: Steam only writes
    // a game's compatibility entry once someone picks a tool for it.
    let Ok(block) = descend(&mut document.value, rest, Missing::Fail) else {
        return Ok(PriorBlock::Absent);
    };

    let entries = keys
        .iter()
        .map(|key| PriorScalar {
            key: (*key).to_owned(),
            value: read(block, key),
        })
        .collect();
    Ok(PriorBlock::Present { entries })
}

/// Puts a block back the way [`capture_block`] found it.
///
/// Only the recorded keys are touched. Anything the owning program has written
/// into the same block since stays, which is the whole reason this exists
/// instead of restoring a pre-image of the file.
pub fn restore_block(text: &str, path: &[&str], prior: &PriorBlock) -> Result<String, VdfError> {
    let mut document: Vdf<'static> = Vdf::from(parse(text)?).into_owned();
    let rest = descend_path(&document, path)?;

    match prior {
        PriorBlock::Absent => remove_block(&mut document, rest),
        PriorBlock::Present { entries } => {
            let Ok(block) = descend(&mut document.value, rest, Missing::Fail) else {
                // Steam dropped the block itself. Recreating it to hold values
                // it no longer tracks would be inventing state, not undoing.
                return Ok(document.to_string());
            };
            for entry in entries {
                match &entry.value {
                    Some(value) => write(block, &entry.key, value),
                    None => {
                        block.0.remove(entry.key.as_str());
                    }
                }
            }
        }
    }

    Ok(document.to_string())
}

/// Takes the block the run added back out of its parent.
fn remove_block(document: &mut Vdf<'static>, path: &[&str]) {
    let Some((name, parent_path)) = path.split_last() else {
        return;
    };
    // Already gone is success: an undo has to be safe to run twice.
    if let Ok(parent) = descend(&mut document.value, parent_path, Missing::Fail) {
        parent.0.remove(*name);
    }
}

/// Checks the document's own top-level key and returns the rest of the path.
///
/// A file whose root is not what the caller expects is not the file this should
/// be editing, the same check the write side makes.
fn descend_path<'a>(
    document: &Vdf<'static>,
    path: &'a [&'a str],
) -> Result<&'a [&'a str], VdfError> {
    let (root, rest) = path.split_first().ok_or_else(|| VdfError::MissingSection {
        section: String::new(),
    })?;
    if document.key != *root {
        return Err(VdfError::MissingSection {
            section: (*root).to_owned(),
        });
    }
    Ok(rest)
}

#[cfg(test)]
#[path = "vdf_prior_test.rs"]
mod vdf_prior_test;
