//! The GPU, and the two files gameready writes inside the user's own session.

/// Where the kernel lists every DRM device, one directory per card.
pub const SYS_CLASS_DRM: &str = "/sys/class/drm";

/// A DRM device's PCI vendor id, relative to its `/sys/class/drm` entry.
pub const DEVICE_VENDOR: &str = "device/vendor";

/// The directory systemd reads user environment fragments from, relative to
/// `$HOME`.
///
/// Read once when the user's session starts, which is why a step writing here
/// has to tell the user the change lands on the next login rather than now.
pub const ENVIRONMENT_D_DIR: &str = ".config/environment.d";

/// The fragment holding the shader cache settings.
pub const SHADER_CACHE_CONF: &str = "99-gameready-shader-cache.conf";

/// gamemode's per-user configuration file, relative to `$HOME`.
///
/// gamemode reads this in preference to `/etc/gamemode.ini`, so gameready can
/// configure it without root and undo it by deleting one file.
pub const GAMEMODE_INI: &str = ".config/gamemode.ini";
