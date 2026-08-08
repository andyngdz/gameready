# SkipReason serializes with tag = reason

Case:
`SkipReason` in `improvement/domain/outcome.rs` carries `#[serde(tag = "reason")]`, so no variant may have a field named `reason`. `CouldNotTell { detail }` is named that way for this and no other reason; `serde_derive` fails the build with "variant field name `reason` conflicts with internal tag" plus a cascade of unsatisfied Serialize bounds that point at `Outcome`, not at the variant that caused it.

`Probe` is tagged `state`, so `Probe::Unknown { reason }` is fine.
