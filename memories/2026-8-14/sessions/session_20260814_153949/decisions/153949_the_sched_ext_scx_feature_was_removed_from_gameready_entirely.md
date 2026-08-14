# The sched_ext/scx feature was removed from gameready entirely

Decision:
The whole CPU scheduler feature — `core.sched.scx-lavd` and `core.repo.scx-ppa`
plus every bit of its machinery — was cut in one removal. The user called the
scheduler "garbage" and approved a hard removal:

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

Live machine (andy, Ubuntu PPA stack) at removal time:
- `scxctl` is NOT installed. `scx.service` runs `/usr/sbin/scx_lavd` via the
  gameready drop-in `/etc/systemd/system/scx.service.d/10-gameready.conf`
  (`SCX_SCHEDULER_OVERRIDE=/usr/sbin/scx_lavd`).
- `/etc/default/scx` already names `SCX_SCHEDULER=scx_cosmos`, so the ONLY
  thing forcing lavd is that drop-in. To switch to cosmos: remove the
  drop-in, `systemctl daemon-reload`, `systemctl restart scx`, and verify with
  `cat /sys/kernel/sched_ext/root/ops`.
- Don't run `gameready rollback` on the old binary for that run: its scheduler
  undo calls `scxctl stop`, which does not exist.
