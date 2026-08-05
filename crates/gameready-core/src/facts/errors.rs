//! Errors from probing the system.

use thiserror::Error;

use crate::exec::ExecError;

/// Why a system fact could not be established.
///
/// Probing failing is fatal to a run rather than something to work around: a
/// step that does not know the kernel version cannot decide whether it applies,
/// and guessing is how a tool ends up changing the wrong thing.
#[derive(Debug, Error)]
pub enum FactsError {
    #[error("could not probe {what}")]
    Probe {
        what: &'static str,
        #[source]
        source: ExecError,
    },

    #[error("`{release}` is not a kernel release this can compare")]
    KernelRelease { release: String },

    #[error("/etc/os-release has no {missing} field")]
    OsRelease { missing: &'static str },

    /// The distro is real but outside what gameready knows how to drive. Named
    /// separately from a parse failure so the message can say which distro and
    /// what it claimed to be like, which is what a bug report needs.
    #[error("`{id}` (ID_LIKE=`{id_like}`) is not a distro gameready supports")]
    UnsupportedDistro { id: String, id_like: String },
}
