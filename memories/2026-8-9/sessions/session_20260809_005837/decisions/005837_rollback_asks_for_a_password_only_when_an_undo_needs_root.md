# Rollback asks for a password only when an undo needs root

Decision:
`gameready rollback` used to prime sudo for every run with anything to undo. It now calls `escalation.ask()` only when `RollbackPlan::needs_root()` is true or `--purge-packages` was given. `Undo::privilege()` reads the privilege the file operations recorded at write time; everything else (sysctl, sysfs, systemd, apt) is root by definition.

Reason: the same rule the init flow already follows. Asking for a password to delete a file the user owns teaches them to type it without reading what asked.

This is what makes the rollback snapshot possible: the snapshot seeds a journal with one user-privilege `FileWritten` and runs against the real machine (`Reads::ThisMachine`), because `FixtureRunner` refuses every write and a fixture rollback can only ever report that it put nothing back.
