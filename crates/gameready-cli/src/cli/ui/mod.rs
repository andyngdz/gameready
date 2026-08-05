//! Turning run reports into text. Nothing here writes to stdout; `main` does.

mod summary;

pub use summary::render;
