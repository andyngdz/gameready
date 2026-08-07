# Steam CompatToolMapping needs priority 250 and Valve's own tool names

Case:
Case: pinning a Proton build writes `InstallConfigStore/Software/Valve/Steam/CompatToolMapping/<appid>` in `~/.steam/steam/config/config.vdf`, with `name`, `config` and `priority`.

Two values are not guessable from the code:

- `priority` must beat the machine-wide default. Steam files "run everything through this" under appid `0` at priority 75 (read off this machine's own config.vdf), so an entry below that is stored and then ignored. Per-game entries use 250.
- A community build's `name` is its directory in `compatibilitytools.d`. Valve's own builds are not there at all: Steam knows them by internal names such as `proton_experimental`, `proton_9`, `proton_hotfix`. Read them with `strings ~/.steam/steam/appcache/appinfo.vdf | grep -o "proton_[a-z0-9_]*" | sort -u`.

Verified that `keyvalues-parser` round-trips a real 24KB config.vdf: quote count rose by exactly the 14 the new entry adds, and the escaped-JSON `WebStorage` blob came back byte for byte apart from one tab in its separator, which is the indentation normalisation `steam/service/vdf.rs` already documents.

Related: [[scx-ships-in-two-packages-and-apt-needs-a-ppa]]

Links:
- [[scx-ships-in-two-packages-and-apt-needs-a-ppa]] → memories/2026-8-7/sessions/session_20260807_023848/edge-cases/023848_scx_ships_in_two_packages_and_apt_needs_a_ppa.md
