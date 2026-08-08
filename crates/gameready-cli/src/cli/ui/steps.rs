//! Where the user is in the questions a run has to ask.

use std::fmt::Write as _;

use console::style;

use crate::cli::ui::layout::Section;

/// The header over each question, counting it against the run's total.
///
/// The count is the point. A question with no idea how many follow it reads as
/// open-ended, and the whole flow is built on the opposite promise: everything
/// is asked before anything changes, and there is an end to it.
pub struct Steps {
    total: usize,
    asked: usize,
}

impl Steps {
    /// A counter for a run that has `total` questions to put.
    ///
    /// The total is an upper bound worked out before the first answer, because
    /// most of the questions only exist depending on the answer to an earlier
    /// one. A run that ends at "step 3 of 4" is a user who picked nothing at
    /// step 1; a run that reached "step 5 of 4" would be a bug.
    #[must_use]
    pub const fn of(total: usize) -> Self {
        Self { total, asked: 0 }
    }

    /// Heads the next question, and counts it.
    ///
    /// `caution` is the warning that belongs in the header rather than in the
    /// body: the packages question is the one thing in the run a rollback
    /// cannot take back, and that has to be visible before the question is
    /// read, not after.
    pub fn heading(&mut self, caution: Option<&str>) {
        self.asked = self.asked.saturating_add(1);
        if console::user_attended_stderr() {
            eprint!("{}", self.rendered(caution));
        }
    }

    /// The header as text, so a test can read what a terminal would show.
    fn rendered(&self, caution: Option<&str>) -> String {
        let mut out = String::new();
        let mut s = Section::new(&mut out);
        let label = self.label(caution);
        // Writing into a String cannot fail, and a header that could not be
        // built is not worth failing a run over.
        let _ = s.blank().and_then(|()| s.banner(&label));
        out
    }

    /// The label itself, coloured for how much attention it wants.
    fn label(&self, caution: Option<&str>) -> String {
        let mut label = format!("STEP {} OF {}", self.asked, self.total);
        if let Some(warning) = caution {
            let _ = write!(label, " · {}", warning.to_uppercase());
            return style(label).yellow().bold().to_string();
        }
        style(label).blue().bold().to_string()
    }
}

#[cfg(test)]
#[path = "steps_test.rs"]
mod steps_test;
