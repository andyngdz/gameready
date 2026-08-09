# Most gamemode.ini settings are already its defaults

Case:
Upstream `example/gamemode.ini` shows only one setting worth gameready writing, which is why `core.gamemode.config` writes a single line.

Already gamemode's own defaults, so writing them restates a default and leaves gameready owning it:

- `ioprio=0` (it already puts clients at BE/0)
- `inhibit_screensaver=1`
- `disable_splitlock=1`

That last one is why `core.sysctl.split-lock` stands down when gamemode is on PATH: gamemode clears `kernel.split_lock_mitigate` for the duration of any gamemoderun client and restores it after, and every launch option gameready writes starts with gamemoderun. The sysctl step exists for games not started that way.

Two more that look useful and are not:

- `softrealtime=auto` needs SCHED_ISO, which no mainline kernel carries.
- `renice=20` (negated into nice -20) is the one real lever, but gamemoded reads the *client process's* groups, so it does nothing unless the user is in the `gamemode` group AND has logged back in since. Probe with `id -nG`, not `/etc/group`.

NVIDIA's `__GL_SHADER_DISK_CACHE_SKIP_CLEANUP=1` is a related trap: it removes the cache size limit rather than raising it, so pairing it with a chosen size lets the cache grow until the disk fills. `core.gpu.shader-cache` sets the size alone.

See [[scx-lavd-is-sold-as-less-stutter-under-load]] for the same honesty rule applied to a different step.
