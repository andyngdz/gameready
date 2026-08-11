# Testing a tray click needs the CLI reinstalled

Case:
Case: The tray resolves `gameready` through `runner.which`, so a click runs
whatever is first on PATH (`~/.local/bin/gameready`), never the repo build. A
fix in the workspace changes nothing about what the click does until
`cargo install --path crates/gameready-cli --force` has run.

The same applies to the tray itself: its journal watcher refreshes data, it does
not reload the program, so a running tray keeps reporting what its own binary
knows. Verifying a tray change end to end takes both a CLI reinstall and a tray
restart.

Symptom when this is missed: the row's note does not change after an update,
which reads as a broken refresh. Check the journal instead. A run recorded there
with `outcome: failed` and no `changed` event means the refresh fired and the
run genuinely did nothing.

Related: [[proton-ge-asset-and-directory-names-come-off-the-release-not-the-tag]]

Links:
- [[proton-ge-asset-and-directory-names-come-off-the-release-not-the-tag]] → memories/2026-8-12/sessions/session_20260812_021255/edge-cases/021255_proton_ge_asset_and_directory_names_come_off_the_release_not_the_tag.md
