//! What a step says about itself while it works.

/// One report from a step that is part-way through applying.
///
/// One type rather than one callback per kind, because the run loop that
/// forwards these holds the event sink mutably and cannot lend it out twice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Doing {
    /// A named sub-phase started: downloading, verifying, extracting.
    Phase(String),

    /// Bytes have landed out of a known total.
    ///
    /// Only a step that knows what it is fetching before it starts can report
    /// this, which is why it is not the only kind of progress there is.
    Bytes { done: u64, total: u64 },
}
