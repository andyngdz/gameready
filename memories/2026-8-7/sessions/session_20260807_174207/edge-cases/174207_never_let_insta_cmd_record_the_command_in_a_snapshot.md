# Never let insta_cmd record the command in a snapshot

Case:
Case: `insta_cmd::assert_cmd_snapshot!` writes an `info:` block into the `.snap` file listing the program, args and every env var set with `Command::env`. `insta::Settings::add_filter` does not reach that block, only the snapshot body.

Every path this repo's CLI tests pass is absolute (`GAMEREADY_FAKE_ROOT`, `HOME`, the temp state dir), so committing those snapshots would commit the home directory of whoever generated them, which the identity rule forbids.

`crates/gameready-cli/tests/snapshots.rs` runs the binary with plain `std::process::Command`, formats exit code plus stdout plus stderr itself, and snapshots that string with `insta::assert_snapshot!`. Filters then work, and nothing but output reaches the file. `insta_cmd` is still used for `get_cargo_bin`.
