# Phase 4 screens are built to full mock fidelity, per-step copy included

Decision:
Decision: For the remaining Phase 4 screens (explain, list-games, rollback, selftest, summary, help, the init run), match the mock verbatim including the new per-step copy the codebase does not have yet. The user chose full fidelity over shipping structure-with-existing-fields and deferring the copy.

What that means in practice, per the mock at docs/design/GameReady-CLI.dc.html:
- explain one adds "Gets" (the benefit, split from the mechanism in rationale) and "Costs" (download size, derivable from the plan's InstallPackages approx_bytes rather than a new field) and an undo note ("hands the CPU straight back, no reboot").
- explain index groups System vs Per game, titles "N tunings, in the order a run works through them", and gives each step a descriptive blurb distinct from both name() and short_name() (e.g. "Daemons that fight gamemode").
- doctor is already done (short_name is the identity label there).

Blockers to know before starting explain: the index "Per game" group needs the game steps enumerated, but core_steps() lists only core steps and there is no game_steps() registry yet; SteamLaunchOptions/SteamProton exist as types. Add a registry or enumerate them explicitly. Related: [[redesign-mock-lives-in-docs-design-and-the-claude-design-project]].

Links:
- [[redesign-mock-lives-in-docs-design-and-the-claude-design-project]] → memories/2026-8-8/sessions/session_20260808_143256/references/143256_redesign_mock_lives_in_docs_design_and_the_claude_design_project.md
