//! Which GPU is in the machine, and how that decides the shader cache setting.

/// The size gameready gives the on-disk shader cache.
///
/// Both drivers default to 1GB, which a single large title can fill on its own.
/// Once it is full the driver evicts, and the shaders it evicted are recompiled
/// on the next launch: the stutter this step exists to remove.
///
/// 12GB is the value the gaming-tuned distro guides converge on. It is a
/// ceiling, not an allocation, so a small library never uses it.
const CACHE_GIGABYTES: u32 = 12;

/// One environment variable and the value to give it.
pub type CacheSetting = (&'static str, String);

/// The GPU vendors gameready can recognise from a PCI id.
///
/// Only three because only three matter here: the split that decides the
/// settings is NVIDIA's proprietary driver against everything on Mesa. AMD and
/// Intel are kept apart anyway so the evidence line can name the actual card.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuVendor {
    /// NVIDIA's proprietary driver, which has its own cache variables.
    Nvidia,

    /// AMD on Mesa.
    Amd,

    /// Intel on Mesa.
    Intel,
}

/// What looking for a GPU found.
///
/// A separate state rather than an absent vendor, because "no card gameready
/// knows" is a normal outcome the step reports and not a failure to read
/// anything: a container has no DRM nodes at all, and a card from a fourth
/// vendor gets no setting rather than a guessed one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectedGpu {
    /// A vendor gameready has cache settings for.
    Recognised(GpuVendor),

    /// Nothing this step knows how to configure.
    Unrecognised,
}

impl GpuVendor {
    /// Reads a vendor out of the PCI id sysfs exposes, such as `0x10de`.
    #[must_use]
    pub fn from_pci_id(raw: &str) -> DetectedGpu {
        match raw.trim().to_ascii_lowercase().as_str() {
            "0x10de" => DetectedGpu::Recognised(Self::Nvidia),
            "0x1002" => DetectedGpu::Recognised(Self::Amd),
            "0x8086" => DetectedGpu::Recognised(Self::Intel),
            _ => DetectedGpu::Unrecognised,
        }
    }

    /// The name shown in a probe's evidence line.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Nvidia => "NVIDIA",
            Self::Amd => "AMD",
            Self::Intel => "Intel",
        }
    }

    /// The environment variables that raise this driver's cache ceiling.
    ///
    /// NVIDIA also has `__GL_SHADER_DISK_CACHE_SKIP_CLEANUP`, and it is
    /// deliberately not set: it does not raise the limit, it removes it, so the
    /// cache would grow until the disk filled. A ceiling the user agreed to is
    /// the point of the step.
    #[must_use]
    pub fn cache_settings(self) -> Vec<CacheSetting> {
        match self {
            Self::Nvidia => vec![
                ("__GL_SHADER_DISK_CACHE", "1".to_owned()),
                (
                    "__GL_SHADER_DISK_CACHE_SIZE",
                    (u64::from(CACHE_GIGABYTES) * 1_000_000_000).to_string(),
                ),
            ],
            // Mesa reads a suffixed size and assumes gigabytes without one.
            Self::Amd | Self::Intel => {
                vec![("MESA_SHADER_CACHE_MAX_SIZE", format!("{CACHE_GIGABYTES}G"))]
            }
        }
    }
}

#[cfg(test)]
#[path = "gpu_test.rs"]
mod gpu_test;
