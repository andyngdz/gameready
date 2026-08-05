//! Why a unit query could not be answered.

use thiserror::Error;

use crate::exec::ExecError;

/// A unit query that produced no answer at all.
///
/// Distinct from "the unit is absent", which is an answer. A caller that cannot
/// tell must not treat that as "nothing is there".
#[derive(Debug, Error)]
pub enum SystemdError {
    /// No `systemctl` on `PATH`. A container or a non-systemd init, where every
    /// unit question is unanswerable rather than answered no.
    #[error("systemctl is not on PATH, so unit state cannot be read")]
    Unavailable,

    #[error("could not read the state of `{unit}`")]
    Query {
        unit: String,
        #[source]
        source: ExecError,
    },
}
