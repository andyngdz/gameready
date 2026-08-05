//! Names shared across the exec feature.

/// The escalator users already have a credential cache for.
///
/// Named once because the lookup, the command construction, and the rendering
/// of a privileged command all reference it. A rename that fixed only one site
/// would silently build `doas -v` style nonsense.
pub const SUDO: &str = "sudo";

/// OpenBSD's simpler escalator, shipped by some Arch users.
pub const DOAS: &str = "doas";

/// systemd's escalator, present on very recent systems.
pub const RUN0: &str = "run0";

/// polkit's escalator, which prompts through the desktop agent.
pub const PKEXEC: &str = "pkexec";
