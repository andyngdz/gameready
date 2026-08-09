# System tunings and per-game tunings are separate menu folders

Decision:
`core_steps()` tunes the machine; only `game_steps()` (launch options, Proton
pin) belongs to one game. The tray keeps them in two submenus and never files
a system tuning under a game's name.

`tray::sweep_game(runner, app_id, user_games)` builds the per-game rows the
same way `init` does: `discover_setups` -> `targets_for` /
`compat_targets_for` -> `SteamLaunchOptions::new` / `SteamProton::new`, then
probes. So a row says what a run would write, not what the profile contains.

The gamemode watcher reports only the name and appid, never the rows: it runs
on the thread serving D-Bus signals and reading Steam's config files there
would block it. The main loop fills them in.
