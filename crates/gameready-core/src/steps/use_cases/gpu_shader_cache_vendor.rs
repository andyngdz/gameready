//! Finds which GPU vendor's driver this machine renders with.

use std::path::Path;

use crate::exec::CommandRunner;
use crate::improvement::StepError;
use crate::steps::constants::{DEVICE_VENDOR, SYS_CLASS_DRM};
use crate::steps::domain::{DetectedGpu, GpuVendor};

/// Reads the vendor of the first recognised card under `/sys/class/drm`.
///
/// `/sys/class/drm` also lists one entry per connector (`card0-DP-1`) and each
/// carries the same `device/vendor` as its card, so a name containing a hyphen
/// is skipped to avoid reading one GPU several times.
///
/// A machine with both an integrated and a discrete GPU reports whichever the
/// kernel enumerated first, which is not always the one a game runs on. Both
/// vendors' settings are inert on the other's driver, so the cost of guessing
/// wrong is a variable nothing reads rather than a wrong setting.
pub fn detect_vendor(runner: &dyn CommandRunner) -> Result<DetectedGpu, StepError> {
    // A machine with no `/sys/class/drm` at all fails the listing, and that is
    // the same answer as an empty one: there is no GPU here to configure. It is
    // not treated as an error because the step has nothing to restore either
    // way, unlike a sysctl step that must read a value before overwriting it.
    let Ok(entries) = runner.read_dir(Path::new(SYS_CLASS_DRM)) else {
        return Ok(DetectedGpu::Unrecognised);
    };

    for entry in entries {
        let Some(name) = entry.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !name.starts_with("card") || name.contains('-') {
            continue;
        }

        let vendor_path = entry.join(DEVICE_VENDOR);
        if !runner.path_exists(&vendor_path) {
            continue;
        }

        let raw = runner
            .read_to_string(&vendor_path)
            .map_err(StepError::Exec)?;
        if let DetectedGpu::Recognised(vendor) = GpuVendor::from_pci_id(&raw) {
            return Ok(DetectedGpu::Recognised(vendor));
        }
    }

    Ok(DetectedGpu::Unrecognised)
}

#[cfg(test)]
#[path = "gpu_shader_cache_vendor_test.rs"]
mod gpu_shader_cache_vendor_test;
