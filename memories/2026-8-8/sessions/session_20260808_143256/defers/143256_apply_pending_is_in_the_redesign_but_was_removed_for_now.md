# Apply --pending is in the redesign but was removed for now

Deferred:
Deferred: the redesign mock (section 05, "something else owns the file") keeps `gameready apply --pending`: when Steam is open, launch options skip with "Quit Steam, then: gameready apply --pending", and a later invocation applies the queued work.

I removed SkipReason::SteamRunning in commit c476b56 because nothing constructed it and the flag did not exist; the user approved removing it "for now". So the redesign still wants this feature and it is not built.

Unblock: when launch-options writing becomes a real engine step (today it is done by the CLI's LaunchChoice::carry_out, not the step engine), have it emit a Steam-open skip and add an `apply --pending` command that reads the last run's queued steps and applies them. Rebuild SkipReason::SteamRunning at that point. Related: [[redesign-mock-lives-in-docs-design-and-the-claude-design-project]].

Links:
- [[redesign-mock-lives-in-docs-design-and-the-claude-design-project]] → memories/2026-8-8/sessions/session_20260808_143256/references/143256_redesign_mock_lives_in_docs_design_and_the_claude_design_project.md
