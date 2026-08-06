# I/O scheduler step is a latency tweak, not an FPS one

Decision:
Decision:
`core.io.scheduler` (crates/gameready-core/src/steps/use_cases/io_scheduler.rs)
sets none for NVMe, mq-deadline for SATA SSD, bfq for a spinning disk. Kept
deliberately even though its payoff is narrow: it reduces load-time and
streaming stutter under disk pressure and does not raise frame rate. On an
NVMe-only machine (the common gaming case, and this dev machine) it reports
AlreadyApplied and does nothing. The user affirmed keeping it. Do not "fix" it
to chase FPS, and do not drop it as useless: bfq on an HDD is the real win.

How it applies: writes the live sysfs scheduler per disk (undoable via
Change::SysfsWrite) plus a udev rule at /etc/udev/rules.d/60-gameready-ioscheduler.rules
for reboot persistence. No `udevadm trigger`; the direct sysfs write covers the
current boot.

Testing gotcha: MockRunner.write_sysfs stores the raw value written ("none"),
but real sysfs reads back a bracketed line ("[none] mq-deadline"). So an
apply-then-verify chain on one mock will not parse after apply. Verify tests
seed a fresh runner with the post-apply bracketed lines instead of chaining.

Real kernel proof needs `gameready selftest --step core.io.scheduler` on hardware
with sudo; it cannot run in this sandbox. Covered by mock lifecycle tests plus a
fail-at sweep. Related: [[sudo-rs-blocks-agent-testing]]
