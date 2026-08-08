# Snapshotting a screen that needs a run id needs TZ pinned

Case:
`crates/gameready-cli/tests/snapshots.rs` pins `TZ=UTC` alongside `NO_COLOR`, `TERM` and `COLUMNS`. A `RunId` is a ULID carrying the time it was made, and the rollback screen prints it in the reader's own zone, so without `TZ` the snapshot is whoever took it.

The rollback fixture also uses a hardcoded ULID (`FIXED_RUN`) rather than `RunId::generate()`, for the same reason.
