# Contributing

## Build

```bash
git clone https://github.com/andyngdz/gameready
cd gameready
cargo build
```

Needs Rust 1.90 or newer.

## Test

```bash
cargo test                                       # unit and snapshot tests
cargo test --features docker-tests -- --ignored  # per-distro containers
gameready selftest --all                         # real system, real rollback
```

See [TESTING.md](TESTING.md) for what each layer covers.

After changing CLI output, review snapshots with `cargo insta review`.

## Add a game profile

1. Create `games/<Name>/game.toml`:

```toml
name = "Example Game"
steam_appid = 123456

[launch]
gamemode = true
mangohud = true

[proton]
prefer = "GE-Proton"
```

2. Run `cargo test` to confirm it parses.

The profile is embedded into the binary at build time. Users can override it
at `~/.config/gameready/games/<Name>/game.toml`.

## Add a step

1. Create a file in `crates/gameready-core/src/steps/use_cases/`.
2. Implement `Improvement` and `CoreImprovement` (see an existing step for the
   shape: `probe`, `plan`, `apply`, `verify`, `rollback`).
3. Wire it into `core_steps()` in `crates/gameready-core/src/steps/service.rs`.
4. Write tests in a sibling `<name>_test.rs` file covering at least: probe
   when absent, probe when already applied, apply emitting the expected
   commands, and rollback restoring prior state.

## Code style

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- No `unwrap()` or `expect()` in library code; those belong in tests.
- Test bodies go in a sibling `_test.rs` file, not inline in the source.

## Pull requests

- One step per PR when touching the engine.
- Run `cargo test` before pushing; CI runs the same plus the distro matrix.
- Snapshot changes show up in the diff. If the change is intentional, commit
  the updated `.snap` files.

## License

Contributions are licensed under GPL-3.0-or-later, matching the project.
