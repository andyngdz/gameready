# Non-game Steam apps filtered via appinfo.vdf

Decision:
Decision:
The scan drops any installed Steam app whose `appinfo.vdf` `common/type` is not
`Game`. Reads `appcache/appinfo.vdf` through the `steam-vdf-parser` crate in
`crates/gameready-core/src/infra/steam/appinfo.rs` (`NonGameApps`), and the scan
skips those appids.

The filter is "not Game", not "== DLC". Cyberpunk 2077 Bonus Content (appid
1495710) is typed `Music`, not `DLC`; a DLC-only check missed it. Verified
on-disk: the three real games read `Game`, Valve tools read `Tool`, the bonus
reads `Music`. This replaces the old defer that assumed the type was `DLC`.

Degrade-safe by design: a missing, unreadable, or unparseable appinfo.vdf yields
an empty set, so the scan lists every app as before rather than failing. Never
let a bad read hide a real game. A parse failure logs at `tracing::debug`.

Fragility to watch: `steam-vdf-parser` 0.1.2 is pre-1.0, single-maintainer, ~2k
downloads, and reads Valve's binary appinfo format, which Valve bumps without
notice (v40 to v41 recently). If Valve ships a format the crate cannot read, the
filter silently turns off (falls back to showing everything) rather than
breaking; the debug log is the signal. Coverage is two `#[ignore]` real-Steam
tests, run locally, not in CI.

Related: [[the-plan-file-is-not-the-status-of-this-repo]]

Links:
- [[the-plan-file-is-not-the-status-of-this-repo]] → memories/2026-8-6/sessions/session_20260806_140216/decisions/140216_the_plan_file_is_not_the_status_of_this_repo.md
