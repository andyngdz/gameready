# Every failure states broke, now, and the one command

Decision:
Section 05 of the redesign is implemented as `Outcome::trouble()` in `improvement/domain/trouble.rs`, returning `Trouble { broke, now, fix }`. The copy lives in core with the rest of the outcome vocabulary; the CLI's `ui/trouble.rs` only lays it out.

Reason: the failure shapes are properties of the outcome, not of the screen. `doctor`, the summary, and anything later that reports a run cannot then describe the same failure differently.

Consequences worth knowing:

- A step that went wrong breaks out of the summary's dotted-row shape entirely. Three sentences after a leader would make the one line that matters look like the ones the reader can skip.
- `Probe::Unknown` settles as `Skipped { CouldNotTell }`, not `NotApplicable`. A probe that could not read has not established that this machine cannot take the step.
- `Probe::Conflict` carries `yours: Option<String>`, the one command that hands the setting back. Only the step knows it: `systemctl disable --now tuned.service` for a daemon, nothing at all for a scheduler somebody else loaded.
- The mock's sixth shape, "something else owns the file" (Steam open, `gameready apply --pending`), has no representation. See [[apply-pending-is-in-the-redesign-but-was-removed-for-now]].

Links:
- [[apply-pending-is-in-the-redesign-but-was-removed-for-now]] → memories/2026-8-8/sessions/session_20260808_143256/defers/143256_apply_pending_is_in_the_redesign_but_was_removed_for_now.md
