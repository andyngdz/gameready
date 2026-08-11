# Proton-GE update hint is a green separate state, not amber

Decision:
Decision: An installed-but-outdated Proton-GE reads as green with a separate update-hint line, never amber.

Proton-GE amber on the tray read as "not installed", which was confusing: GE-Proton11-3 was installed, only the latest GitHub tag (GE-Proton11-5) was missing. The user chose "green + separate hint" over other options.

So Probe gained UpdateAvailable { installed, latest }: any GE-Proton dir in the compat dir counts as "has Proton-GE" (green dot, `•` already-set mark in doctor), and the version hint is a distinct tray note line plus the doctor phrase "installed X, Y available". ProbeStatus::UpdateAvailable maps to Ink::Live and Mark::AlreadySet deliberately, sharing the green with Set.

Run/selftest/explain behavior is preserved: an outdated install still upgrades on run, still cycles on selftest, still shows "Would do install <latest>" in explain. Only the colour and the note changed.

**Why:** amber meant "ready to install", but the machine already had Proton-GE; the signal misled. The update hint had to stay visible, just not as a warning colour.

**How to apply:** keep UpdateAvailable green; if a future step needs the amber "install this" signal, it is Probe::Applicable, not an outdated existing install. Do not "fix" the mapping back to amber.
