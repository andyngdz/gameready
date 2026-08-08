# A run reports once, and asks for a password only when it needs one

Decision:
`gameready init` writes Steam's per-game settings after the core sweep, because Steam has to be closed first. Those steps are now folded into the same `RunReport` (`LaunchChoice::carry_out` returns `SteamSettingsDone::Written(RunReport)` and `InitRequest::carry_out` extends `report.steps` with it) rather than rendered by a screen of their own.

Reason: the summary's verdict was computed from the core report alone, so a run that had just written launch options still closed with "Your machine was already set up", and the launch lines landed after the summary's closing separator. The old `LaunchReport` also printed one sentence per applied step, so a run setting launch options and a Proton pin said "Launch options set. Steam is restarting." twice.

Separately, `escalation.ask()` is now gated on `RunPlan::needs_root()` in both `init` and `apply`. The plan screen's `needs_password()` had computed the same predicate since it was written, and its own doc comment claimed the escalation used it; it did not, so a run of nothing but Steam config still prompted for a password.

Not covered by a test on either side: the end-to-end Steam write. `init_test.rs` seeds no Steam directory, so `SteamSettingsDone::Written` is never reached there.
