# Testing

gameready has four test layers, each covering what the layers below it cannot.

## Unit tests (`cargo test`)

Step logic runs against `MockRunner`, an in-memory fake system. Tests cover the
command sequence, journal records, and rollback path without root or a real
machine.

Parser tests use fixtures for `/etc/os-release` (Arch, Debian, Ubuntu, Fedora,
and derivatives), `appmanifest_*.acf`, `libraryfolders.vdf`, and `game.toml`.

The proptest suite generates random apply-then-rollback sequences and asserts
that the mock filesystem returns to its starting state.

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

## Distro containers (`cargo test --features docker-tests -- --ignored`)

A static musl binary is built and mounted into Arch, Debian, Ubuntu, and Fedora
containers. Tests cover:

- Distro detection on a real `/etc/os-release`
- Package manager selection
- `gameready doctor` and `gameready list-games` running to completion

CI runs this matrix on every push to main and every pull request. The musl
build avoids glibc version skew across the four images.

## Real hardware (`gameready selftest`)

```bash
gameready selftest --all
gameready selftest --step core.sysctl.max-map-count
```

Runs the full step lifecycle on the real system: probe, plan, apply, verify,
rollback, verify reverted. This is the only way to test:

- Writing to `/proc/sys` and `/sys`
- Writing Steam config files against a real installation

`selftest` needs the privileges the steps need (sudo for kernel parameters,
user for Steam config). It rolls back everything it applies, so the system
ends where it started.

## What each layer covers

```
Layer       Scope                           Needs
----------- ------------------------------- -----------------
Unit        logic, parsing, journal math    nothing
Snapshot    CLI screens, help text          nothing
Container   distro detection, pkg names     Docker
Selftest    kernel state, Steam config      real machine, sudo
```

Container tests cannot write to `/proc/sys` or `/sys` (read-only in Docker,
and sysctl namespaces do not cover `vm.*`). Selftest cannot run in CI because
GitHub Actions runners share a kernel. The two layers together cover what
neither can alone.
