# Rollback restores Steam config without closing Steam first

Deferred:
Deferred: `gameready rollback` puts `config.vdf` and `localconfig.vdf` back while Steam is running, and Steam overwrites both from memory when it next exits. The restore is then silently undone.

The apply path already handles this: `infra/steam/write_settings.rs` calls `shutdown` before writing and `start` afterwards. Nothing in `rollback/service/` or `cli/commands/rollback.rs` does the same.

Found while testing the Proton pin on real hardware on 2026-08-07: the write and restart worked, and the rollback only stuck because Steam was closed by hand first.

Unblock: have the rollback engine close Steam when the undo plan touches a path under the Steam directory, then restart it, the same way the write path does. Not a blind shutdown on every rollback: most runs touch nothing of Steam's and closing a running game client for a sysctl undo would be worse than the bug.

Related: [[steam-compattoolmapping-needs-priority-250-and-valve-s-own-tool-names]]

Links:
- [[steam-compattoolmapping-needs-priority-250-and-valve-s-own-tool-names]] → memories/2026-8-7/sessions/session_20260807_174207/edge-cases/174207_steam_compattoolmapping_needs_priority_250_and_valve_s_own_tool_names.md
