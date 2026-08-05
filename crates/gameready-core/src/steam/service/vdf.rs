//! Reading and writing values inside a Steam config file.
//!
//! Parsing is `keyvalues-parser`'s job. The round trip is not byte-for-byte:
//! the crate stores each block in a `BTreeMap`, so keys come back sorted, and
//! it normalises the indentation. Checked against a real 89KB `localconfig.vdf`
//! before relying on it: all 994 key-value pairs survive unchanged and a second
//! render is identical to the first. Steam reads the file as a key-value store
//! and rewrites it in its own layout when it exits, so neither ordering nor
//! indentation carries meaning.

use std::borrow::Cow;

use keyvalues_parser::{Obj, Value, Vdf, parse};

use crate::steam::errors::VdfError;

/// What one edit found and would change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    /// The whole file, re-rendered with the new value in place.
    pub text: String,
    /// What the value was before, so the caller can show it and journal it.
    pub previous: String,
}

/// What setting a scalar would do to the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetResult {
    /// The value is already exactly what was asked for, so the caller can skip
    /// the write rather than journal a change that changes nothing.
    AlreadySet,
    /// The file needs rewriting.
    Changed(Edit),
}

/// Sets a scalar under `path`, adding the key when it is absent.
///
/// `path` names the blocks to descend, outermost first, and includes the
/// document's own top-level key.
pub fn set_scalar(
    text: &str,
    path: &[&str],
    key: &str,
    value: &str,
) -> Result<SetResult, VdfError> {
    let mut document: Vdf<'static> = Vdf::from(parse(text)?).into_owned();

    let (root, rest) = path.split_first().ok_or_else(|| VdfError::MissingSection {
        section: String::new(),
    })?;
    if document.key != *root {
        return Err(VdfError::MissingSection {
            section: (*root).to_owned(),
        });
    }

    let block = descend(&mut document.value, rest)?;
    let previous = read(block, key);
    if previous.as_deref() == Some(value) {
        return Ok(SetResult::AlreadySet);
    }

    block.0.insert(
        Cow::Owned(key.to_owned()),
        vec![Value::Str(Cow::Owned(value.to_owned()))],
    );

    Ok(SetResult::Changed(Edit {
        text: document.to_string(),
        previous: previous.unwrap_or_default(),
    }))
}

/// Walks into nested blocks, failing on the first one the file does not have.
fn descend<'a>(
    value: &'a mut Value<'static>,
    path: &[&str],
) -> Result<&'a mut Obj<'static>, VdfError> {
    let mut block = as_block(value)?;

    for section in path {
        let next = block
            .0
            .get_mut(*section)
            .and_then(|values| values.first_mut())
            .ok_or_else(|| VdfError::MissingSection {
                section: (*section).to_owned(),
            })?;
        block = as_block(next)?;
    }
    Ok(block)
}

/// A value that has to be a block for the path to continue through it.
fn as_block<'a>(value: &'a mut Value<'static>) -> Result<&'a mut Obj<'static>, VdfError> {
    match value {
        Value::Obj(block) => Ok(block),
        // A scalar where a block was expected means the path names something
        // real but of the wrong shape, which is not a file this should write to.
        Value::Str(_) => Err(VdfError::NotABlock),
    }
}

/// The current value of a key, when it is present and is a scalar.
fn read(block: &Obj<'static>, key: &str) -> Option<String> {
    match block.0.get(key)?.first()? {
        Value::Str(text) => Some(text.clone().into_owned()),
        Value::Obj(_) => None,
    }
}

#[cfg(test)]
#[path = "vdf_test.rs"]
mod vdf_test;
