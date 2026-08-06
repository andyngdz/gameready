# DLC entries get launch options once every game is taken

Deferred:
Deferred:
`gameready init --yes` now takes every installed game, so a Steam DLC entry gets `gamemoderun %command%` written into launch options it will never use.

Measured on this machine: `appmanifest_1495710.acf` is "Cyberpunk 2077 Bonus Content", a DLC, and it is indistinguishable from the real game by anything in the manifest. Both carry `StateFlags 4`, `LastPlayed 0`, an `installdir`, and a `SizeOnDisk`. Size differs (1GB vs 66GB) but is not a discriminator.

`is_valve_tool` in `crates/gameready-core/src/steam/domain/tools.rs` filters Proton, the Steam Linux Runtimes, Steamworks, and SteamVR by appid and name prefix. DLC does not match any of them, and there is no prefix that would catch it without also catching real games.

Harm is low: the entry never launches, so the option is inert, and rollback removes it like any other. The visible cost is a wrong line in the summary claiming a DLC was tuned.

Unblock: the type field (`game` / `dlc` / `tool`) lives in Steam's `appinfo.vdf`, a binary format that neither `keyvalues-parser` nor `steamlocate` reads today. Either add a reader for it, or find a signal in the `.acf` that separates a DLC from a game and extend `is_valve_tool`. Do not filter on `SizeOnDisk`.

Related: [[dxvk-async-is-dead-and-must-not-go-in-launch-options]]

Links:
- [[dxvk-async-is-dead-and-must-not-go-in-launch-options]] → memories/2026-8-6/sessions/session_20260806_132607/decisions/132607_dxvk_async_is_dead_and_must_not_go_in_launch_options.md
