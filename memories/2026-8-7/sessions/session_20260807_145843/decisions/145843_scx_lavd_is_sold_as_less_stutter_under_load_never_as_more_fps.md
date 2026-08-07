# Scx_lavd is sold as less stutter under load, never as more FPS

Decision:
Decision:
`core.sched.scx-lavd` describes itself as steadier frame times when something
else wants the CPU, and explicitly says a game that leans on one or two cores
may get slower. It is not default-on and never claims an FPS gain.

Reason:
Researched 2026-08-07 against primary sources, because the measured evidence is
thinner than the reputation.

- The Igalia LPC 2024 design talk (`lpc.events/event/18/contributions/1713`) has
  no benchmark numbers at all across its 14 slides. It explains the mechanism: a
  frame is a chain of threads waking each other, most tasks run under 1 msec, and
  LAVD ranks by wake/wait frequency so a thread the chain is blocked on runs
  first.
- Phoronix reports Igalia's own result as "better or similar performance to
  EEVDF" on average FPS and 1% lows. Not "much better".
- sched-ext/scx issue #296 (opened 2024-05-18, closed "not planned"): about 20%
  LOWER FPS in Starcraft 2 and Diablo 3 on a Ryzen 5950X. Games that lean on one
  or two cores. Closed without a fix.
- sched-ext/scx issue #3340 (opened 2026-02-18, closed): scx_lavd in performance
  mode pegged CPU at 98-100% and leaked ~34GB on an Intel Core Ultra 9 275HX,
  CachyOS 6.19.2, scx-scheds 1.0.20. scx_bpfland was fine on the same box.
- The scx_lavd README says it mainly targets single CCX / single socket systems.

A 25 to 32 FPS (32%) figure circulates from the OSS NA 2024 deck via the
Phoronix forums. The forum page 403s and the deck was not reachable, so treat it
as unverified.

The step also never writes a config file to make the scheduler persist. It loads
at runtime and evaporates on reboot, so there is exactly one thing to undo and
the undo is instant. See [[scx-ships-in-two-packages-and-apt-needs-a-ppa]].

Links:
- [[scx-ships-in-two-packages-and-apt-needs-a-ppa]] → memories/2026-8-7/sessions/session_20260807_023848/edge-cases/023848_scx_ships_in_two_packages_and_apt_needs_a_ppa.md
