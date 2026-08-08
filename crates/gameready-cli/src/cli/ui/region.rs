//! The one live line at the bottom of a run, and what it is doing.
//!
//! Owns every indicatif bar the CLI makes, so the rest of the progress code
//! decides what to say and this decides how it is drawn.

use indicatif::{ProgressBar, ProgressStyle};

/// A step that is working, with nothing to measure.
const SPINNER: &str = "  {spinner:.blue} {msg}";

/// A step that is fetching something whose size is known.
///
/// One line: a template with a newline in it is not laid out as two lines, it
/// is padded to the terminal width, which puts the bar off at the right margin
/// with a field of blank between. Decimal units rather than binary, because
/// every other screen sizes a download in MB and 178.05 MiB beside "178 MB"
/// reads as two different files.
const COUNTING: &str =
    "  {spinner:.blue} {msg} {bar:20.blue/blue} {decimal_bytes}/{decimal_total_bytes}";

/// The characters a filled, current, and empty bar cell are drawn with.
///
/// A rule rather than a row of blocks, and no brackets around it. The bar sits
/// on one line between a step name and a byte count, where a solid block reads
/// as a second thing on the screen rather than as the line it is drawn on.
const CELLS: &str = "━━─";

/// How often a spinner redraws itself.
const TICK: std::time::Duration = std::time::Duration::from_millis(80);

/// The live region: at most one bar, replaced rather than stacked.
#[derive(Debug, Default)]
pub(crate) struct LiveRegion {
    bar: Option<ProgressBar>,

    /// The total the current bar is counting towards, when it is counting one.
    /// Kept so a download reporting every 64 KiB moves the bar it already has
    /// rather than building three thousand of them.
    counting: Option<u64>,
}

impl LiveRegion {
    /// Shows a message with a spinner, replacing whatever was there.
    pub(crate) fn spin(&mut self, message: String) {
        self.clear();
        self.bar = Some(Self::started(SPINNER, message, None));
    }

    /// Changes what the live line says, keeping the spinner that is there.
    ///
    /// A spinner torn down and rebuilt for every message reads as several
    /// separate waits rather than as one thing taking a while. A bar counting
    /// something is replaced rather than relabelled: its total belongs to the
    /// transfer that is now over.
    pub(crate) fn say(&mut self, message: String) {
        match (&self.bar, self.counting) {
            (Some(bar), None) => bar.set_message(message),
            _ => self.spin(message),
        }
    }

    /// Shows how much of a known total has arrived.
    ///
    /// The first report builds the bar and every one after it moves the same
    /// bar, so the region stays one line's worth of screen however often the
    /// transfer reports.
    pub(crate) fn count(&mut self, message: &str, done: u64, total: u64) {
        if self.counting != Some(total) {
            self.clear();
            self.bar = Some(Self::started(COUNTING, message.to_owned(), Some(total)));
            self.counting = Some(total);
        }
        if let Some(bar) = &self.bar {
            bar.set_position(done);
        }
    }

    /// Takes the region down, leaving nothing behind.
    pub(crate) fn clear(&mut self) {
        self.counting = None;
        if let Some(bar) = self.bar.take() {
            bar.finish_and_clear();
        }
    }

    /// Replaces the live line with a finished one and leaves it on screen.
    ///
    /// Returns whether the bar took the message. A line that wraps has to be
    /// printed instead: `finish_with_message` pads whatever it is given to one
    /// bar's width, which turns a two-line row into a wall of spaces.
    pub(crate) fn settle(&mut self, message: &str) -> bool {
        let counting = self.counting.take();
        let Some(bar) = self.bar.take() else {
            return false;
        };
        // A counting bar's style still holds the bar itself, so finishing it
        // with a row would leave the row trailing a full bar of the thing that
        // just ended.
        if counting.is_some() || message.contains('\n') {
            bar.finish_and_clear();
            return false;
        }
        bar.finish_with_message(message.to_owned());
        true
    }

    /// One bar, styled and started.
    fn started(template: &str, message: String, total: Option<u64>) -> ProgressBar {
        let bar = total.map_or_else(ProgressBar::new_spinner, ProgressBar::new);
        if let Ok(style) = ProgressStyle::with_template(template) {
            bar.set_style(style.progress_chars(CELLS));
        }
        bar.enable_steady_tick(TICK);
        bar.set_message(message);
        bar
    }
}

/// What is on screen, for tests.
///
/// A live region has no production reader: everything that draws into it
/// already knows what it just drew. These exist so a test can assert on the
/// screen rather than on the calls that made it.
#[cfg(test)]
impl LiveRegion {
    /// Whether anything is on screen right now.
    pub(crate) const fn is_live(&self) -> bool {
        self.bar.is_some()
    }

    /// What the live line says.
    pub(crate) fn saying(&self) -> Option<String> {
        self.bar.as_ref().map(ProgressBar::message)
    }

    /// The total the live bar is counting towards, if it is counting one.
    pub(crate) const fn counting(&self) -> Option<u64> {
        self.counting
    }
}

#[cfg(test)]
#[path = "region_test.rs"]
mod region_test;
