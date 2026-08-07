# Scx ships in two packages, and apt needs a PPA

Case:
Verified 2026-08-07 against the Arch package index, the Launchpad API, the Fedora COPR API, and upstream `sched-ext/scx` INSTALL.md.

`scx-scheds` carries only scheduler binaries. Arch `scx-scheds` 1.1.2-1 (repo `extra`) ships 17 `scx_*` binaries plus `scxtop` and no systemd unit at all. `scx_loader`, `scxctl`, `scx_loader.service`, `org.scx.Loader` D-Bus files and the polkit policy live in a separate package, Arch `scx-tools` 1.1.2. The Fedora COPR `bieszczaders/kernel-cachyos-addons` splits them the same way (`scx-scheds` 1.1.2-2, `scx-tools` 1.1.2-1). A step that loads a scheduler needs both packages, not one.

`scx_loader` also left the main repo: it is now developed at `github.com/sched-ext/scx-loader`, reading `/etc/scx_loader/config.toml` or `/etc/scx_loader.toml`.

The Ubuntu primary archive has no scx at all: Launchpad `getPublishedSources` returns `total_size: 0` for `scx-scheds` on noble, plucky, questing and resolute. `ppa:arighi/sched-ext` does carry it, under source name `scx` (not `scx-scheds`), at 1.1.1-1 for resolute and 1.0.21~rc2-1 for noble. Adding that PPA is a persistent system change and needs its own journal entry, the same as the Fedora COPR.

No `sched-ext/scx` GitHub release ships a binary asset; the last five (v1.1.2 back to v1.0.19) carry only source tarballs.

`scx_lavd` is the right scheduler for gaming; Valve ships it on SteamOS.

The plan file at ~/.claude/plans/we-re-gonna-make-a-swift-moon.md states "apt has nothing" and "install package scx, run scx_lavd via scx_loader". Both are wrong on the detail above. See [[the-plan-file-is-not-the-status-of-this-repo]].

Links:
- [[the-plan-file-is-not-the-status-of-this-repo]] → memories/2026-8-6/sessions/session_20260806_140216/decisions/140216_the_plan_file_is_not_the_status_of_this_repo.md
