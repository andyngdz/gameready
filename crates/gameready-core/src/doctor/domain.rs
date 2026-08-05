//! What the doctor reports.

use serde::{Deserialize, Serialize};

/// One thing the doctor found that the user should know about.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Warning {
    /// What was found, in one sentence.
    pub finding: String,

    /// Why it matters, in plain language.
    pub explanation: String,

    /// What to do about it.
    pub suggestion: String,
}

impl Warning {
    #[must_use]
    pub fn new(
        finding: impl Into<String>,
        explanation: impl Into<String>,
        suggestion: impl Into<String>,
    ) -> Self {
        Self {
            finding: finding.into(),
            explanation: explanation.into(),
            suggestion: suggestion.into(),
        }
    }
}
