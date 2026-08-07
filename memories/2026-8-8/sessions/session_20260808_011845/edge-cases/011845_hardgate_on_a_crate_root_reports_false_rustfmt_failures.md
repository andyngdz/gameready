# Hardgate on a crate root reports false rustfmt failures

Case:
`hardgate rust --scope crates/<crate>/src/main.rs` invokes bare `rustfmt --check`
with no `--edition`, so it applies edition-2015 import ordering and reports a diff
for every file in the crate, including files the change never touched.

This repo is edition 2024. `rustfmt --check --edition 2024 <file>` and
`cargo fmt --all --check` both come back clean on the same tree.

Reproduce the false positive on an unmodified checkout before treating one of
these as your own regression:

```
git stash && rustfmt --check crates/gameready-cli/src/main.rs; git stash pop
```

Directory scopes (`crates/gameready-cli/src/cli`, `crates/gameready-core/src/run`)
go through `cargo fmt` instead and do not have the problem, so prefer a directory
scope over a single `main.rs`.
