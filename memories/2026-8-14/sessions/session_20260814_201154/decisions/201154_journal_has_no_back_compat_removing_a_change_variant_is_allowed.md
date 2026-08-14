# Journal has no back-compat: removing a Change variant is allowed

Decision:
Decision: gameready does not keep read-back `Change`/`Undo` variants for a
removed feature. Deleting a step deletes its journal variants too.

Reason: the repo has exactly one user, on one machine. Carrying a variant only
so an old `journal.jsonl` still parses buys nothing and keeps a dead vocabulary
alive in `journal/domain/change.rs`, `undo.rs` and `rollback/service/perform.rs`.
Confirmed by the user on 2026-08-14 when the sched_ext removal raised it.

The consequence is real and accepted: `Change` is internally tagged
(`#[serde(tag = "type")]`) and `journal::load` (`journal/service.rs:186`) uses
`?`, so ONE line with a dropped variant makes the whole file `Corrupt` and
`gameready rollback` fails with "could not read the journal" for every run in
it, not just the affected one. After removing a variant, delete or edit the
offending lines out of `~/.local/state/gameready/journal.jsonl` on the live
machine, and undo whatever those lines recorded by hand.

See [[the-sched-ext-scx-feature-was-removed-from-gameready-entirely]].

Links:
- [[the-sched-ext-scx-feature-was-removed-from-gameready-entirely]] → memories/2026-8-14/sessions/session_20260814_153949/decisions/153949_the_sched_ext_scx_feature_was_removed_from_gameready_entirely.md
