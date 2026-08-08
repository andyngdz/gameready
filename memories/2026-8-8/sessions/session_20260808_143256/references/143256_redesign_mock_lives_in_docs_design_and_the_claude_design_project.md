# Redesign mock lives in docs/design and the Claude Design project

Source:
The Phase 4 redesign spec is `docs/design/GameReady-CLI.dc.html` (committed db7f248). It is the authoritative source for every screen's copy and layout: foundations/marks, the init run (probe, 4 questions, agreed plan, live region, summary), the try-it prototype, the reading commands (doctor, explain one, explain index, list-games, rollback, selftest, --help), and section 05 the six failure shapes.

Voice: second person to the user plus the tool as "I" ("Looking at your machine", "I need your password once", "I skip rather than guess"). Marks: ✔ applied, • already set, ✘ failed, – skipped, ! warning, ↻ re-checked, ⠹ running. Header says gameready 0.2.0.

If the local copy is lost again: it is a Claude Design project, id 2c9cae64-549c-410d-b428-a878651f2af4, file "GameReady CLI.dc.html", re-fetchable via the DesignSync MCP (get_file). The v0.1.1 "before" snapshot it was designed from is at uploads/gameready-cli-design-data.json in the same project. DesignSync list_projects does NOT show this project (it lists only design-system projects); pass the id directly to list_files/get_file. Related: [[a-step-reading-sys-needs-a-fixture-file-or-its-snapshot-is-non-deterministic]].

Links:
- [[a-step-reading-sys-needs-a-fixture-file-or-its-snapshot-is-non-deterministic]] → memories/2026-8-8/sessions/session_20260808_135634/edge-cases/135634_a_step_reading_sys_needs_a_fixture_file_or_its_snapshot_is_non_deterministic.md
