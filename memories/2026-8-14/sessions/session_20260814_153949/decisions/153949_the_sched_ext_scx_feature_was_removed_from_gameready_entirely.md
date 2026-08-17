# The sched_ext/scx feature was removed from gameready entirely

Decision:
The whole CPU scheduler feature — `core.sched.scx-lavd` and `core.repo.scx-ppa`
plus every bit of its machinery — was cut in one removal. The user called the
scheduler "garbage" and approved a hard removal:

Actual reason, user-reported: while gaming under `scx_lavd`, **audio got
wrecked** (glitching). That matches lavd's documented trade-off — it reorders
thread wakeups for frame pacing, which can starve latency-sensitive audio
threads (PipeWire). Treat "lavd wrecked my audio" as a live lead if the
scheduler ever comes back.

- Deleted: `scx_lavd`, `scx_lavd_loader`, `scx_lavd_packages`, `scx_ppa`,
  `scx_ppa_pin`, `scx_state`, `steps/domain/sched_ext.rs`,
  `steps/constants/scx.rs`, and the integration test
  `scx_ppa_unlocks_lavd.rs`.
- Journal was hard-removed, not kept dormant: `Change::ScxScheduler`,
  `Change::AptRepository`, `Undo::RestoreScxScheduler`,
  `Undo::RemoveAptRepository` and their perform arms are gone. Old journal
  files carrying `scx_scheduler` or `apt_repository` records now fail to load
  — the user accepted that.
- `DependencyKind::Feature` went too; it only ever served sched_ext.
- The takeover/conflict UI stayed (CPU governor and conflicts still use it)
  but its wording was genericized off "is scheduling the CPU".

Consequence:
The removal stranded files on every machine that had already applied the
feature, with no rollback path left in any build. On the user's box, verified
2026-08-14 after the removal, two orphans survived:

- `/etc/apt/preferences.d/99-gameready-scx.pref` (written by 0.1.1,
  `step=core.repo.scx-ppa`)
- `/etc/systemd/system/scx.service.d/10-gameready.conf` (written by 0.2.3,
  `step=core.sched.scx-lavd`)

Both were inert by then — the arighi PPA was already out of
`/etc/apt/sources.list.d/` and `scx` was not installed — but nothing in the
tool could see or remove them, because the undo variants went with the code.
They were deleted by hand. No cleanup path was added to `doctor`, so anyone
who ran gameready 0.2.4 or older with scx applied still has both files.

Live machine (redacted user, Ubuntu PPA stack) at removal time:
- `scxctl` is NOT installed. `scx.service` runs `/usr/sbin/scx_lavd` via the
  gameready drop-in `/etc/systemd/system/scx.service.d/10-gameready.conf`
  (`SCX_SCHEDULER_OVERRIDE=/usr/sbin/scx_lavd`).
- `/etc/default/scx` already names `SCX_SCHEDULER=scx_cosmos`, so the ONLY
  thing forcing lavd is that drop-in. To switch to cosmos: remove the
  drop-in, `systemctl daemon-reload`, `systemctl restart scx`, and verify with
  `cat /sys/kernel/sched_ext/root/ops`.
- Don't run `gameready rollback` on the old binary for that run: its scheduler
  undo calls `scxctl stop`, which does not exist.

See [[journal-has-no-back-compat-removing-a-change-variant-is-allowed]].

Links:
- [[journal-has-no-back-compat-removing-a-change-variant-is-allowed]] → memories/2026-8-14/sessions/session_20260814_201154/decisions/201154_journal_has_no_back_compat_removing_a_change_variant_is_allowed.md
