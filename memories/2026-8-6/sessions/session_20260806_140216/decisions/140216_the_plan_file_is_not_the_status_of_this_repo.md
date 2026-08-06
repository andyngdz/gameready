# The plan file is not the status of this repo

Decision:
Decision:
Check the code for what exists. Do not read `~/.claude/plans/we-re-gonna-make-a-swift-moon.md` as a record of what is built.

Reason:
The plan was written before PR #6, #7, and #8 and has drifted from `main` in at least three places. It lists `--print-launch-options` as unbuilt M4 work, but the capability shipped as `LaunchChoice::ShowForCopying` in `crates/gameready-cli/src/cli/ui/launch_choice.rs`. It puts the `inquire` game picker in M6; that shipped in #7. It asks for 8-10 game profiles, which the gamemode-by-default decision replaced.

The failure mode this causes: grepping for the plan's *name* for a thing, finding nothing, and proposing to build what already exists under a different name. Searching a string is not reading the code that owns the behaviour. Search for what the thing does.

Related: [[dxvk-async-is-dead-and-must-not-go-in-launch-options]]

Links:
- [[dxvk-async-is-dead-and-must-not-go-in-launch-options]] → memories/2026-8-6/sessions/session_20260806_132607/decisions/132607_dxvk_async_is_dead_and_must_not_go_in_launch_options.md
