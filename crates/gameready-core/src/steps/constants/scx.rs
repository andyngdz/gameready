//! sched_ext: where the kernel reports it, the PPA that carries it on Ubuntu,
//! and the two ways it gets loaded.

/// Where the kernel reports whether a sched_ext scheduler is attached.
///
/// Reads `disabled` when the kernel is scheduling on its own. A kernel built
/// without sched_ext has no such file at all, which is how the step tells
/// "nothing loaded" apart from "cannot load anything".
pub const SCHED_EXT_STATE: &str = "/sys/kernel/sched_ext/state";

/// What [`SCHED_EXT_STATE`] reads when nothing is attached.
pub const SCHED_EXT_DISABLED: &str = "disabled";

/// Where the kernel names the attached scheduler.
///
/// The `root/` directory only exists while a scheduler is attached, so
/// [`SCHED_EXT_STATE`] is always read first.
pub const SCHED_EXT_OPS: &str = "/sys/kernel/sched_ext/root/ops";

/// The scheduler gameready loads, as `scxctl` spells it.
///
/// Bare `lavd`, not `scx_lavd`: the loader takes the short name and prefixes it
/// when it runs the binary.
pub const LAVD_SCHEDULER: &str = "lavd";

/// The PPA that carries scx for Ubuntu.
///
/// The Ubuntu primary archive has no scx at any release. This PPA is maintained
/// by Andrea Righi, who also maintains scx upstream.
pub const SCX_PPA: &str = "ppa:arighi/sched-ext";

/// The `Origin` that PPA stamps on its Release file, used to pin it.
///
/// Read from the live Release file on 2026-08-07. A wrong value here would
/// silently produce a pin that matches nothing, which is the failure mode worth
/// naming: the repository would be added at full priority.
pub const SCX_PPA_ORIGIN: &str = "LP-PPA-arighi-sched-ext";

/// The pin that stops that PPA from supplying anything except scx.
///
/// A new file, never an edit, like every other `/etc` file gameready writes.
pub const SCX_PPA_PIN: &str = "/etc/apt/preferences.d/99-gameready-scx.pref";

/// The tool that adds and removes an Ubuntu PPA.
///
/// Used rather than writing the sources file by hand because it also fetches
/// the signing key over HTTPS from Launchpad, and because `--remove` is then a
/// real inverse rather than a guess at which files were written.
pub const ADD_APT_REPOSITORY_BIN: &str = "add-apt-repository";

/// The flag that takes a repository back off instead of adding one.
pub const APT_REMOVE: &str = "--remove";

/// The flag that stops apt tooling asking a question nobody is there to answer.
///
/// gameready asks everything before it starts, so a prompt appearing mid-run
/// would be one the contract says cannot happen.
pub const APT_ASSUME_YES: &str = "--yes";

/// The unit the Ubuntu `scx` package ships to run a scheduler at boot.
///
/// Its presence is how the step tells an Ubuntu machine apart from an Arch or
/// Fedora one: those get `scx_loader` and `scxctl` in a separate `scx-tools`
/// package, and Ubuntu gets neither.
pub const SCX_UNIT_PATH: &str = "/usr/lib/systemd/system/scx.service";

/// The unit's name, for `systemctl`.
pub const SCX_SERVICE_NAME: &str = "scx";

/// The drop-in that points that unit at scx_lavd.
///
/// A new file rather than an edit of `/etc/default/scx`, which the package
/// owns. The unit reads `SCX_SCHEDULER_OVERRIDE` before its own default, so a
/// drop-in wins without touching the package's file at all.
pub const SCX_UNIT_DROPIN: &str = "/etc/systemd/system/scx.service.d/10-gameready.conf";

/// The scheduler binary the Ubuntu package installs.
pub const SCX_LAVD_BIN: &str = "/usr/sbin/scx_lavd";

/// The environment variable the shipped unit checks before its own default.
pub const SCX_SCHEDULER_OVERRIDE: &str = "SCX_SCHEDULER_OVERRIDE";

/// The tool that loads and unloads a sched_ext CPU scheduler.
///
/// A command-line client for `scx_loader`, which owns the D-Bus interface and
/// the polkit rule. Going through the loader rather than running `scx_lavd`
/// directly means gameready does not have to ship a unit of its own, and a user
/// who later drives the loader by hand is not fighting a second owner.
pub const SCXCTL_BIN: &str = "scxctl";
