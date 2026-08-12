# Core.proton.ge reports failed after a successful install

Deferred:
`core.proton.ge` ends `outcome: failed` on every recent run in
`~/.local/state/gameready/journal.jsonl`, including the run that actually
extracted the build.

The lead is in the run's own `changed` event, which records
`dir_tree_installed` at `compatibilitytools.d/GE-Proton11-5` while the
directory on disk is `GE-Proton11-5-x86_64`. The step looks to be verifying a
path it derived from the tag rather than the one it wrote, which is the same
tag-versus-release split [[proton-ge-asset-and-directory-names-come-off-the-release-not-the-tag]]
already covers for asset names.

Unblock: reproduce with `gameready init`, then read the `core.proton.ge`
entries in the journal and compare the recorded path against
`ls ~/.steam/root/compatibilitytools.d`.

Nothing depends on this: the install itself works, and the pin resolves off a
real directory listing rather than the recorded path.

Links:
- [[proton-ge-asset-and-directory-names-come-off-the-release-not-the-tag]] → memories/2026-8-12/sessions/session_20260812_021255/edge-cases/021255_proton_ge_asset_and_directory_names_come_off_the_release_not_the_tag.md
