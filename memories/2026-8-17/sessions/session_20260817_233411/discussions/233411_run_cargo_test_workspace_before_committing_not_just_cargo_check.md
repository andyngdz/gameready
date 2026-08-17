# Run cargo test workspace before committing, not just cargo check

Topic:
On gameready, verify every change (version bumps included) with
`cargo test --workspace --all-features`, not just `cargo check`. A bump was
committed with only `cargo check` run, and that was flagged as insufficient.

**Why:** `cargo check` proves it compiles, not that behavior still holds. A
version-string bump still changes the `--help` snapshot, so the full suite is
the done-signal before the commit, not after being asked. (A v0.2.7 bump broke
`help_says_what_every_command_is_for` exactly this way.)

**How to apply:** Before committing any code or version change, run
`cargo test --workspace --all-features` and report the `test result:` line.
Treat "it compiles" as insufficient evidence of done.
