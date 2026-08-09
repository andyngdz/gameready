# The tray refreshes by watching the journal, not on a timer

Decision:
Decision: the tray re-probes the machine when a gameready run writes the
journal, not on a 60s timer. A timer re-ran a dozen subprocesses to learn
nothing, since a tuning does not change unless something changes it, and every
change lands in `journal.jsonl` and fsyncs before the CLI returns.

`crates/gameready-tray/src/infra/journal.rs` watches with inotify and filters
on the filename `journal.jsonl`. It watches the state *directory*, not the
file: a machine that never ran has no journal yet, and a watch on a missing
path fails. One sweep per batch of appends, because a run writes a line per
step.

Cross-crate contract: the tray's `state_dir()`
(`crates/gameready-tray/src/infra/watchers.rs`) must resolve to the same path
the CLI writes to, `state_paths()` in `crates/gameready-cli/src/main.rs`. Both
use `ProjectDirs::from("", "", "gameready").state_dir()` with a `data_dir()`
fallback. Change one without the other and the tray watches an empty directory
and never refreshes. Catching a run under sudo in another terminal is free,
because it is the same file either way.

Links: [[Two zbus signal iterators on one thread deadlock the second]]

Links:
- [[Two zbus signal iterators on one thread deadlock the second]] → memories/2026-8-9/sessions/session_20260809_155703/edge-cases/155703_two_zbus_signal_iterators_on_one_thread_deadlock_the_second.md
