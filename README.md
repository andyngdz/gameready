# gameready

A command line tool that applies gaming-related system tuning on Linux and can
undo it.

Status: the engine, all planned steps, and the full `init` flow work.
See [milestones](#milestones).

## Scope

gameready applies changes in these categories:

- kernel parameters via `/etc/sysctl.d`
- block device I/O schedulers via a udev rule
- sched_ext schedulers, loaded at runtime
- packages: gamemode, mangohud, gamescope
- Proton-GE into `compatibilitytools.d`
- per-game Steam launch options, environment variables, and Proton version

It does not install kernels, edit `/etc/default/grub`, change GPU drivers, or
add kernel command line parameters.

`gameready doctor` reports three settings it will not apply, and flags them if
they are already present on the system:

| Setting | Why it is not applied |
|---|---|
| `mitigations=off` | Measured gaming effect is around 1% on CPUs from 2020 onward, against Spectre and Meltdown exposure |
| `vm.swappiness=1` | Works against zram, which most distros now enable by default |
| `kernel.sched_latency_ns` and related | Removed in kernel 6.6 with EEVDF; writing them to `/etc/sysctl.d` has no effect |

## Install

Download the static binary from the
[latest release](https://github.com/andyngdz/gameready/releases/latest):

```bash
curl -LO https://github.com/andyngdz/gameready/releases/latest/download/gameready-linux-x86_64
chmod +x gameready-linux-x86_64
sudo mv gameready-linux-x86_64 /usr/local/bin/gameready
```

Or build from source (needs Rust 1.90 or newer):

```bash
git clone https://github.com/andyngdz/gameready
cd gameready
cargo build --release
./target/release/gameready --help
```

Arch users can build from the [PKGBUILD](pkg/arch/PKGBUILD).

## Use

```bash
gameready doctor            # system facts and what is currently applied
gameready apply --dry-run   # the plan, without applying it
gameready apply             # apply
gameready rollback          # undo the last run
gameready selftest          # apply, verify, roll back, verify reverted
```

The process runs as the invoking user. Commands that need root are run
individually through `sudo`, primed once at the start of a run.

## How a step works

Each step implements five methods:

- `probe` reads the current state and decides whether the step applies
- `plan` describes what applying would change
- `apply` makes the change, recording each mutation in the journal first
- `verify` reads the system back and compares against what was written
- `rollback` reverses the changes the step recorded

The executor calls `verify` after `apply`. If a check fails, the step is
reported as failed and rolled back from its own journal records rather than
reported as applied.

`verify` compares system state against the value the step wrote. It does not
measure frame rates.

## Undo

Each run writes to `~/.local/state/gameready/journal.jsonl` before making a
change, plus pre-images of replaced files under `backups/`.

```bash
gameready rollback                    # last run
gameready rollback --run <id>         # a specific run
gameready rollback --purge-packages   # also remove packages the run installed
```

Rollback reverses configuration changes in reverse order, so a runtime value
goes back before the file that persists it is removed.

A file that changed since gameready wrote it is left alone and reported. The
recorded digest is what distinguishes undoing our own work from destroying a
hand edit.

Packages installed during a run stay installed unless `--purge-packages` is
passed, because removing a package is not the inverse of installing one:
dependency cascades, leftover configuration, and other users of the package all
differ from the original operation.

## Supported

Arch (pacman), Debian and Ubuntu (apt), Fedora (dnf). Steam game detection.

## Testing

```bash
cargo test                                       # unit and snapshot tests
cargo test --features docker-tests -- --ignored  # per-distro containers, slow
gameready selftest --all                         # real system, real rollback
```

See [TESTING.md](TESTING.md) for what each layer covers and how to work with
snapshots.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## Milestones

- **M1** engine, journal, rollback, one step (done)
- **M2** system detection, CLI shell, `doctor`, container test matrix (done)
- **M3** package and service steps (done)
- **M4** Steam scanning, per-game profiles, launch options (done)
- **M5** sched_ext, I/O scheduler, swappiness policy, Proton-GE (done)
- **M6** full `init` flow, docs, packaging (done)

## License

GPL-3.0-or-later. See [LICENSE](LICENSE).
