# The panel has its own label, separate from the terminal's

Decision:
`Improvement::bar_name` is the panel's label; `short_name` is the terminal's.
`vm.max_map_count` is right beside a monospace gutter and looks like a typo in
a menu next to "Swappiness". All 15 core steps set `bar_name` explicitly; the
default falls back to `short_name` for anything added later.

A matching `bar_note` for the row's second line was built and then removed: the
tunings moved into submenus, a row became a name and a dot, and nothing read
the note any more. Detail that needs a sentence stays in `gameready doctor`.

Adding a method to every step costs LOC everywhere: `bar_name` pushed three
files past the 250-line gate and each needed a real split
(`cpu_governor_policies`, `scx_lavd_packages`, `memory_swappiness_state`).
Budget for that.
