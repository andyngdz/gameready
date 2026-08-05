//! Why a Steam library could not be read.

use thiserror::Error;

/// A library that could not be scanned.
///
/// Steam being absent is its own variant rather than an empty result: a machine
/// with no Steam and a machine whose Steam is unreadable need different words,
/// and only the second is something the user can act on.
#[derive(Debug, Error)]
pub enum VdfError {
    /// Boxed because the parser's error type is large, and this variant travels
    /// inside `StepError`, which every step's every method returns.
    #[error("this Steam config could not be read as a config file")]
    Malformed {
        #[source]
        source: Box<keyvalues_parser::error::Error>,
    },

    #[error("this Steam config has no `{section}` section, so it is not the file expected here")]
    MissingSection { section: String },

    /// The path names something that exists but holds a value rather than a
    /// block, so descending further would be writing into the wrong shape.
    #[error("this Steam config has a value where a section was expected")]
    NotABlock,
}

impl From<keyvalues_parser::error::Error> for VdfError {
    fn from(source: keyvalues_parser::error::Error) -> Self {
        Self::Malformed {
            source: Box::new(source),
        }
    }
}

/// A library that could not be scanned.
#[derive(Debug, Error)]
pub enum SteamError {
    #[error("no Steam installation was found")]
    NotInstalled,

    #[error("Steam is installed but its library index could not be read")]
    UnreadableLibrary {
        #[source]
        source: steamlocate::Error,
    },

    #[error("could not ask Steam to quit: {detail}")]
    Shutdown { detail: String },

    /// Steam was asked to quit and did not. Its config must not be written now:
    /// Steam would overwrite it on the way out and the change would vanish.
    #[error("Steam was still running after {waited:?}; quit it and run this again")]
    StillRunning { waited: std::time::Duration },

    #[error("no Steam user config was found under `userdata`")]
    NoUserConfig,

    #[error(transparent)]
    Vdf(#[from] VdfError),
}
