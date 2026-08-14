# A sched_ext attach has no event on the Ubuntu PPA stack, so readiness is gated in the unit

Verified 2026-08-14 on the user's Ubuntu machine (scx 1.0.19, scx_lavd 1.1.2):

- The kernel emits no uevent when a sched_ext scheduler attaches: `/sys/kernel/sched_ext/root/` has no `uevent` file and sched_ext never calls kobject_uevent, so udev/inotify are dead ends.
- `scx_lavd` 1.1.2 links no libsystemd (`readelf -d` shows only libelf/libz/libgcc/libm/libc), so it cannot send sd_notify and `Type=notify` cannot be used. `--help` has no notify flag either.
- The Ubuntu PPA ships no `scx_loader`/`scxctl`, so the D-Bus attach event SteamOS relies on (scx_loader sends READY=1) does not exist here.

Consequence: any solution must observe `/sys/kernel/sched_ext/root/ops`, and the wait is not optional. The chosen mechanism is a readiness gate in gameready's own drop-in at `/etc/systemd/system/scx.service.d/10-gameready.conf`:

```
ExecStartPost=/usr/bin/timeout 10 sh -c 'until grep -q ^lavd /sys/kernel/sched_ext/root/ops; do sleep 0.1; done'
```

This makes `systemctl enable --now scx` block until the scheduler actually attaches (the package unit is `Type=simple`, so systemctl would otherwise return the moment the wrapper shell spawns, while the BPF program attaches ~2.8 s later). Timeout exits 124 and fails the unit, which surfaces as a real command failure in the step. Rust-side polling (a previous attempt) was rejected as hacky.

Second gotcha from the same debugging session: `/sys/kernel/sched_ext/root/ops` reports the scheduler name WITH version and target triple (`lavd_1.1.2_x86_64_unknown_linux_gnu`, `cosmos_1.1.5_...`), so any comparison against the short name (`lavd`, `cosmos`) must match the first `_`-segment, and the journal's `previous` for rollback must store the short name (`scxctl switch -s <short>` rejects the versioned string).
