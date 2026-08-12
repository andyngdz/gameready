# Capturing an init prompt needs a pty, and the write path quits real Steam

Case:
Case: `init`'s questions never appear in `--dry-run` (`(Picker::Ask, false)` answers them without asking) and never under `--yes`. Verifying prompt copy therefore needs a real `init` in a pty, which is one keystroke away from mutating the machine.

Two things make that safe, neither obvious from the repo:

- `GAMEREADY_FAKE_ROOT` (used only in `crates/gameready-cli/tests/snapshots.rs`) serves every `/sys`, `/proc`, and command read from `crates/gameready-cli/tests/roots/ubuntu-nvme` through FixtureRunner. Combined with `HOME`, `GAMEREADY_STATE_DIR`, and `GAMEREADY_GAMES_DIR` pointed at temp dirs, a real `init` touches nothing outside them. Games have to be seeded into `$HOME/.steam/steam/steamapps` (libraryfolders.vdf plus one appmanifest each) or `setups` is empty and every game-dependent question is skipped.
- Drive it with `printf '<keys>' | script -qec ./probe.sh /dev/null`. `inquire` needs a pty on stdin, which a plain pipe does not give. Space toggles a multi-select, `\033[B` moves down, `\n` confirms. Set `COLUMNS` to the pty's actual width (80 under `script`): the theme wraps to `COLUMNS`, so a mismatch produces mid-word breaks that look like a wrapping bug and are not.

The hazard: `LaunchChoice::CloseSteamAndWrite` calls `is_running`/`shutdown`, and those run `pgrep`/`steam -shutdown` against the real session regardless of `GAMEREADY_FAKE_ROOT`, because Steam process control does not go through FixtureRunner. Check `pgrep -x steam` first and stay on `ShowForCopying`, or the probe quits the user's Steam.

Related: [[every-question-is-built-through-theme-asked]], [[a-step-reading-sys-needs-a-fixture-file-or-its-snapshot-is-non-deterministic]]

Links:
- [[every-question-is-built-through-theme-asked]] → memories/2026-8-9/sessions/session_20260809_131035/decisions/131035_every_question_is_built_through_theme_asked.md
- [[a-step-reading-sys-needs-a-fixture-file-or-its-snapshot-is-non-deterministic]] → memories/2026-8-8/sessions/session_20260808_135634/edge-cases/135634_a_step_reading_sys_needs_a_fixture_file_or_its_snapshot_is_non_deterministic.md
