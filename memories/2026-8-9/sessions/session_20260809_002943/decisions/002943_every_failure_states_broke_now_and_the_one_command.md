# Only a real failure breaks the summary row shape

Decision:
Section 05's failure vocabulary lives in `Outcome::trouble()` (`improvement/domain/trouble.rs`), returning `Trouble { broke, now, fix }`. The copy lives in core; the CLI lays it out in `ui/rows.rs` via `StepRow`, which owns every summary step line. `ui/trouble.rs`/`WentWrong` is gone.

Reason: the failure shapes are properties of the outcome, not of the screen, so `doctor`, the summary, and anything later report the same failure the same way.

Consequences worth knowing:

- Only `Outcome::Failed` breaks out of the row shape into the three-line block (broke / now / fix). A conflict or could-not-tell is a `Skipped` and stays a single aligned row like its neighbours, because nothing on the machine is in question. `StepRow::write` routes on `matches!(outcome, Outcome::Failed)`, not on whether `trouble()` is `Some` (conflict skips carry a trouble too).
- A conflict that hands back a command (`SkipReason::Conflict.yours`) prints that one command on its own line under the row. The command carries no `❯` prompt glyph, only bold: `rows::copyable()` (was `prompt()`, and the `PROMPT` const in `ui/mod.rs` is gone). The user asked to drop the glyph. The framing sentences ("Your call...", "I left it alone...") are also dropped on a row.
- Helper sub-lines sit under the name, not under the mark: `Section::sub` indents 4 (a marked line spends 2 + one-column glyph + 1 space before its name), so a conflict command and the broke/now/fix lines all line up with the step name above them.
- The name-to-evidence gap is `layout::GAP` (4), shared by both result renderers so they stay aligned: `Section::row` (summary, live region) adds it as trailing padding, `ResultTable` (doctor, rollback, selftest) as the name column's right padding. `table_test` pins the two to the same column, so widen both or neither.
- `Probe::Unknown` settles as `Skipped { CouldNotTell }`, not `NotApplicable`. A probe that could not read has not established the machine cannot take the step.
- The mock's sixth shape, "something else owns the file" (Steam open, `gameready apply --pending`), still has no representation. See [[apply-pending-is-in-the-redesign-but-was-removed-for-now]].

Links:
- [[apply-pending-is-in-the-redesign-but-was-removed-for-now]] → memories/2026-8-8/sessions/session_20260808_143256/defers/143256_apply_pending_is_in_the_redesign_but_was_removed_for_now.md
