//! The apt pin that stops the scx PPA from supplying anything else.
//!
//! Without it a third-party repository can replace any package on the system,
//! including system ones. That is the whole risk a repository step carries, and
//! bounding it is what makes the step something a user can say yes to.

use crate::journal::RunId;
use crate::steps::constants::{managed_header, SCX_PPA_ORIGIN};
use crate::steps::use_cases::scx_ppa::ScxPpa;

/// The pin itself, without the marker.
///
/// Priority 1 is below "not installed", so apt will never take a package
/// from this PPA to satisfy anything. The named package is put back to the
/// ordinary 500, which leaves the PPA able to supply scx and nothing else.
pub fn body(package: &str) -> String {
    format!(
        "Package: *\n\
         Pin: release o={SCX_PPA_ORIGIN}\n\
         Pin-Priority: 1\n\
         \n\
         Package: {package}\n\
         Pin: release o={SCX_PPA_ORIGIN}\n\
         Pin-Priority: 500\n"
    )
}

/// The file as written, carrying the marker `doctor` finds it by.
pub fn file(package: &str, run: RunId) -> String {
    format!(
        "{header}\n{body}",
        header = managed_header(ScxPpa::id_const(), run),
        body = body(package),
    )
}
