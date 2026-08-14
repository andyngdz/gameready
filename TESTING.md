# Testing

gameready has four test layers, each covering what the layers below it cannot.

## Unit tests (`cargo test`)

Step logic runs against `MockRunner`, an in-memory fake system. Tests cover the
command sequence, journal records, and rollback path without root or a real
machine.

Parser tests use fixtures for `/etc/os-release` (Arch, Debian, Ubuntu, Fedora,
and derivatives), `appmanifest_*.acf`, `libraryfolders.vdf`, and `game.toml`.

`tests/rollback_properties.rs` generates random apply-then-rollback sequences
with proptest and asserts the mock filesystem returns to its starting state.
`tests/apply_rollback_cycle.rs` covers the same property by example, including a
kill swept across every command a run issues and a journal truncated mid-write.

## Snapshot tests (`cargo insta review`)

CLI output is pinned with insta snapshots. Every run uses a fake machine:

- `GAMEREADY_FAKE_ROOT` points at `crates/gameready-cli/tests/roots/ubuntu-nvme`,
  which carries a `commands.toml` describing what each shell command returns.
- A temporary `HOME` is built with a synthetic Steam library (three appmanifests
  and one installed Proton build), so no real Steam installation leaks into the
  committed snapshots.

Only the output is snapshotted, never the command that produced it, because the
command includes absolute paths that differ per machine.

To review snapshot changes after editing CLI output:

```bash
cargo insta review
```

## Distro containers (`.github/workflows/ci.yml`, job `distro-matrix`)

A static musl binary is built and mounted into Arch, Debian, Ubuntu, and Fedora
containers. The musl build avoids glibc version skew across the four images.
There is no cargo feature for this: the job runs `docker run` directly, and CI
runs it on every push to main and every pull request.

Each image runs two steps. The first is read-only and needs nothing installed:

- Distro detection on a real `/etc/os-release`
- Package manager selection
- `gameready doctor` and `gameready list-games` running to completion

The second runs `gameready selftest`, which applies each tuning, verifies it,
rolls it back, and verifies it reverted. That one needs a privileged container
and `sudo` installed. Both for reasons worth knowing:

- An unprivileged container mounts `/proc/sys` read-only, and `sysctl -w` there
  prints the new value and **exits 0** while changing nothing. gameready catches
  that by reading `/proc/sys` back, so the run reports a verify failure rather
  than a false success, but no kernel step can actually apply.
- gameready never runs as root. It stays as the invoking user and escalates one
  command at a time, so it needs an escalator on `PATH` even when that user is
  already root. A stock container image ships none.

Fedora and Arch also need `procps-ng` for the `sysctl` binary itself.

A privileged container shares the runner's kernel, so the job really does move
`vm.max_map_count` on the host. selftest puts it back, which is the thing being
tested, and the runner is discarded either way.

## Real hardware (`gameready selftest`)

```bash
gameready selftest
gameready selftest --step core.sysctl.max-map-count
gameready selftest --step game.steam.launch-options
```

Runs the full step lifecycle on the real system: probe, plan, apply, verify,
rollback, verify reverted. This is the only way to test:

- Writing to `/proc/sys` and `/sys`
- Writing Steam config files against a real installation

Every step runs, the two per-game ones included, so "all passed" means all of
them. Steam is quit first when a per-game step is in the list and Steam is up,
then started again after, which is what `init` does to write the settings in
the first place.

A machine with no Steam skips those two rather than failing, the same as a
container skips the shader cache step. Naming one outright is an error instead:
somebody asking for that step specifically is owed the reason it could not run,
not a skip.

`selftest` needs the privileges the steps need (sudo for kernel parameters,
user for Steam config). It rolls back everything it applies, so the system
ends where it started.

## What each layer covers

```
Layer       Scope                           Needs
----------- ------------------------------- -----------------
Unit        logic, parsing, journal math    nothing
Snapshot    CLI screens, help text          nothing
Container   distro detection, apply+rollback Docker
Selftest    kernel state, Steam config      real machine, sudo
```

A container run skips what its image cannot offer, and says so per step rather
than passing quietly: no GPU means no shader cache step, no Steam means no
Proton-GE step, and no systemd means the conflict check cannot read unit state.
Those are the steps only real hardware covers.
