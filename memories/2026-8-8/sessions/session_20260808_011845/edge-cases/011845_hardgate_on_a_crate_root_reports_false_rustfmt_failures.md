# Gameready stays on edition 2021 so both rustfmt callers agree

Decision:
The repo moved from edition 2024 to 2021 on 2026-08-08 to stop `cargo fmt` and a
bare `rustfmt` from disagreeing about import order.

Rustfmt's `style_edition` decides that order, and 2015, 2018 and 2021 all share
one rule while 2024 introduced a new one. Rustfmt defaults to 2015 whatever
Cargo.toml says; only `cargo fmt` reads `edition` and passes it on the command
line. So on edition 2024 the two callers can produce different files, and
`hardgate rust --scope <crate>/src/main.rs` runs the bare one. On 2021 they
cannot disagree at all, whatever the identifiers are.

Measured on this repo before the change, same file, four style editions:

```
2015: 21 files differ    2018: 21    2021: 21    2024: 0
```

Consequence: let chains (`if let Some(x) = a && let Ok(y) = b`) do not compile
here. Two were rewritten as nested `if let` in `infra/exec/mock_runner.rs` and
`mock_runner_impl.rs`. Any edition-2024 feature is off the table until the repo
moves back.

Pinning `style_edition = "2024"` in a `rustfmt.toml` was the other way to fix it
and was rejected: the owner did not want a config file papering over the split.
