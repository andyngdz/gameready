# PPD is not a gamemode-competing daemon

Decision:
power-profiles-daemon (PPD) is no longer reported as fighting gamemode in either summary row.

Reason: modern gamemode cooperates with PPD instead of fighting it. On game start gamemode holds PPD's `performance` profile over its D-Bus API and releases it on exit (FeralInteractive/gamemode #333, #462). So PPD does not "overwrite what gamemode sets", and telling a user to `systemctl disable --now power-profiles-daemon.service` would lose that coordinated switch and their laptop's power management. `tuned` has no such integration, so it stays a real competing daemon.

Two places, both fixed:

- `COMPETING_DAEMONS` (`steps/domain/daemons.rs`, the `Conflicts` step / "Competing daemons" row): PPD removed. Only `ananicy-cpp.service` and `tuned.service` remain.
- `GOVERNOR_DAEMONS` + `cpu_governor.rs` ("CPU governor" row): PPD stays in the list, but is now modeled with `GovernorDaemon { unit, cooperates_with_gamemode }`. `governor_conflict(runner, gamemode_present)` (renamed from `conflicting_daemon`, moved into `cpu_governor_policies.rs` and now returns `Option<Probe>`) skips a cooperating daemon when gamemode is present. So with gamemode present: tuned live -> Conflict(tuned); PPD live -> AlreadyApplied (gamemode drives it). Without gamemode: PPD live -> Conflict(PPD), since a static pin genuinely loses to it.

`governor_conflict` returns the whole `Probe` (not just the unit) so the conflict copy lives beside the daemon logic and `cpu_governor.rs` stays under the 250 LOC RUST029 limit.

`CompetingDaemon.contention` is a dead field, never read; left in place as pre-existing.
