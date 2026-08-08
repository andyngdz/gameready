# A step reading /sys needs a fixture file or its snapshot is non-deterministic

Case:
Case: The CLI snapshot tests run the real binary against GAMEREADY_FAKE_ROOT, which is served by FixtureRunner. FixtureRunner.read_dir / read_to_string resolve paths against the fake root's real filesystem (root.join(path.strip_prefix("/"))) and do NOT fall through to the host. A missing path errors; my read_policies swallows that to an empty Vec.

So any step that probes by reading /sys (or /proc, /etc) decides its snapshot outcome from what tests/roots/ubuntu-nvme actually contains. If the fixture lacks the files, the step reads the degenerate "nothing there" branch, which is stable but usually not the state you want to demonstrate.

Phase 3 hit this: the rewritten core.cpu.governor reads /sys/devices/system/cpu/cpufreq/policy*, which the fixture did not have (it had the old cpu0/cpufreq/scaling_governor the previous always-declining step read). Added crates/gameready-cli/tests/roots/ubuntu-nvme/sys/devices/system/cpu/cpufreq/policy0/{scaling_governor=powersave, scaling_available_governors="performance powersave"} so doctor shows "would apply" deterministically.

How to apply: when a new or changed step reads a system path, seed that path under tests/roots/ubuntu-nvme before regenerating snapshots, or the snapshot pins the empty-read branch. Regenerate with INSTA_UPDATE=always cargo test -p gameready-cli --all-features --test snapshots (cargo insta is not installed). Related: [[never-let-insta_cmd-record-the-command-in-a-snapshot]].

Links:
- [[never-let-insta_cmd-record-the-command-in-a-snapshot]] → memories/2026-8-7/sessions/session_20260807_174207/edge-cases/174207_never_let_insta_cmd_record_the_command_in_a_snapshot.md
