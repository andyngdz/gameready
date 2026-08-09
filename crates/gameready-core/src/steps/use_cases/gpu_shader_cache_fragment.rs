//! Renders and reads back the environment.d fragment the shader cache step owns.

use std::fmt::Write as _;

use itertools::Itertools as _;

use crate::improvement::ImprovementId;
use crate::journal::RunId;
use crate::steps::constants::managed_header;
use crate::steps::domain::GpuVendor;

/// The fragment's full contents, carrying the marker `doctor` looks for.
#[must_use]
pub fn contents(vendor: GpuVendor, step: ImprovementId, run: RunId) -> String {
    let mut rendered = format!(
        "{header}\n\
         # Remove this file or run `gameready rollback` to revert.\n\
         # Read when your session starts, so it applies from your next login.\n",
        header = managed_header(step, run),
    );
    for line in assignments(vendor) {
        let _ = writeln!(rendered, "{line}");
    }
    rendered
}

/// Just the assignments, for the plan screen where the header is noise.
#[must_use]
pub fn preview(vendor: GpuVendor) -> String {
    assignments(vendor).into_iter().join("\n")
}

/// Whether the given file body already sets everything this vendor needs.
#[must_use]
pub fn sets_everything(body: &str, vendor: GpuVendor) -> bool {
    assignments(vendor)
        .iter()
        .all(|line| body.contains(line.as_str()))
}

/// One `KEY=value` line per setting, in the order the vendor lists them.
fn assignments(vendor: GpuVendor) -> Vec<String> {
    vendor
        .cache_settings()
        .into_iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect()
}

#[cfg(test)]
#[path = "gpu_shader_cache_fragment_test.rs"]
mod gpu_shader_cache_fragment_test;
