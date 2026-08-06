# The scan command was dropped from M4

Decision:
Decision:
The planned `scan` command is not built. It was meant to list installed Steam
games and flag ones with no profile as `Unsupported game: <name>`. Both halves
are covered or obsolete:

- `init` already scans Steam through `discover_setups` and shows every installed
  game plus what each gets. `list-games` lists the profiles in the catalog.
- The `Unsupported game` message died with the "tune every game" change: every
  game now gets the `gamemoderun` default, so no game is ever unsupported.

A read-only "list installed Steam games without a picker or sudo" is the only
sliver not covered, and it is too small to justify a command that overlaps
`init --dry-run`. Do not build `scan` unless a real need appears that neither
`init` nor `list-games` serves.

Related: [[the-plan-file-is-not-the-status-of-this-repo]]

Links:
- [[the-plan-file-is-not-the-status-of-this-repo]] → memories/2026-8-6/sessions/session_20260806_140216/decisions/140216_the_plan_file_is_not_the_status_of_this_repo.md
