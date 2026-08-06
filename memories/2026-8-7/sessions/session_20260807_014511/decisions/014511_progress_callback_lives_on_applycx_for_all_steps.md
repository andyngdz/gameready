# Progress callback lives on ApplyCx for all steps

Decision:
Decision: Step sub-phase progress uses a callback on `ApplyCx`, not a step-specific mechanism.

`ApplyCx.progress(&self, message)` calls an `Option<RefCell<Box<dyn FnMut(&str)>>>` attached via `with_progress()`. The executor in `run/service.rs` wires it to emit `RunEvent::StepProgress`, and the CLI's `ProgressView` renders it.

Reason: The user explicitly required progress to be project-wide and reusable across all steps, not just proton-ge. Putting it on `ApplyCx` means any step can call `cx.progress("phase")` with zero extra plumbing.

`RefCell` solves the `&self` vs `FnMut` tension: `progress()` takes `&self` (so it can be called alongside `mutate(&mut self)`), while the callback is `FnMut`. `try_borrow_mut` avoids panics if called recursively.

Steps that complete in under a second (sysctl, swappiness) skip progress calls since the message would flash before anyone reads it. Steps with visible multi-phase work (proton-ge download/verify/extract, gaming_tools package install, io_scheduler rule+sysfs) use it.
