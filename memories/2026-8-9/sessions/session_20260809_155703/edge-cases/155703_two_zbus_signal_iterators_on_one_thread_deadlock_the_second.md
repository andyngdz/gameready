# Two zbus signal iterators on one thread deadlock the second

Case:
Reading two `SignalIterator`s in turn on a single thread only ever advances the
first. `started.next()` blocks until `GameRegistered` arrives, so the reader
never reaches `GameUnregistered` and the tray icon goes green and stays there.

Use one `zbus::blocking::MessageIterator::for_match_rule` matching the whole
interface, and re-query state on any message. Which signal arrived rarely
matters; a single stream is what makes both directions observable.

Lives in `crates/gameready-tray/src/infra/gamemode.rs`.
