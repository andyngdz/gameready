//! Steam's own config vocabulary, and the tools used to fetch Proton-GE.

/// Where Steam keeps per-game settings inside `localconfig.vdf`.
pub const STEAM_APPS_PATH: [&str; 5] =
    ["UserLocalConfigStore", "Software", "Valve", "Steam", "apps"];

/// The key holding a game's launch options.
pub const LAUNCH_OPTIONS_KEY: &str = "LaunchOptions";

/// The name the pre-image of Steam's config is filed under in a run's backups.
pub const LOCAL_CONFIG_BACKUP: &str = "localconfig.vdf";

/// Where Steam records which Proton build runs which game, inside `config.vdf`.
pub const COMPAT_MAPPING_PATH: [&str; 5] = [
    "InstallConfigStore",
    "Software",
    "Valve",
    "Steam",
    "CompatToolMapping",
];

/// The key naming the compatibility tool a game runs under.
pub const COMPAT_NAME_KEY: &str = "name";

/// The key holding tool-specific configuration, which Steam leaves empty.
pub const COMPAT_CONFIG_KEY: &str = "config";

/// The key ranking one mapping entry against another.
pub const COMPAT_PRIORITY_KEY: &str = "priority";

/// The rank a per-game entry needs to beat the machine-wide default.
///
/// Steam files the "run everything through this" setting under appid `0` at
/// priority 75, so a per-game entry below that would be written and then
/// ignored. 250 is what Steam itself writes for a game picked in the
/// Compatibility tab.
pub const COMPAT_GAME_PRIORITY: &str = "250";

/// The rank Steam files its machine-wide default under.
///
/// Left at 75, the value Steam itself writes, so a per-game entry still wins.
/// Raising it to match a game would leave two entries claiming the same game
/// and the winner decided by the order Steam read them in.
pub const COMPAT_MACHINE_WIDE_PRIORITY: &str = "75";

/// The appid Steam files its machine-wide default under.
pub const COMPAT_MACHINE_WIDE_APP_ID: u32 = 0;

/// Valve's own name for Proton Experimental in the mapping.
///
/// Not a directory in `compatibilitytools.d`: Steam installs it as an ordinary
/// app and knows it by this name. Read off this machine's `appinfo.vdf`, which
/// carries the same names for every Proton release.
pub const PROTON_EXPERIMENTAL: &str = "proton_experimental";

/// The name the pre-image of Steam's machine-wide config is filed under.
pub const CONFIG_BACKUP: &str = "config.vdf";

/// GitHub API endpoint for the latest Proton-GE release.
pub const PROTON_GE_LATEST_URL: &str =
    "https://api.github.com/repos/GloriousEggroll/proton-ge-custom/releases/latest";

/// The directory inside a Steam root where custom compatibility tools live.
///
/// Steam discovers tools by scanning for `compatibilitytool.vdf` in each
/// subdirectory on startup.
pub const COMPAT_TOOLS_DIR: &str = "compatibilitytools.d";

/// The manifest file Steam reads to discover a compatibility tool.
pub const COMPAT_TOOL_VDF: &str = "compatibilitytool.vdf";

/// curl binary, used for HTTP fetches.
pub const CURL_BIN: &str = "curl";

/// tar binary, used for archive extraction.
pub const TAR_BIN: &str = "tar";

/// sha512sum binary, used for checksum verification.
pub const SHA512SUM_BIN: &str = "sha512sum";
