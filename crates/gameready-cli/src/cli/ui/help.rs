//! The top-level help card.

use std::fmt;

use console::style;

use crate::cli::ui::layout::Section;

/// What gameready is, in the one line that sits beside the version.
const TAGLINE: &str = "tune a Linux desktop for gaming, and undo it.";

/// The one caveat worth carrying on the front page: a package install is the
/// only thing a rollback cannot take back.
const REVERSIBLE: &str =
    "Everything gameready changes is reversible, except installing a package. It says so before it \
     installs one.";

/// One line of the card: what to type, and what it does.
struct Entry {
    name: &'static str,
    note: &'static str,
}

/// A heading and the commands or flags under it.
///
/// The grouping is the point. Clap lists every subcommand in one flat block,
/// which tells a first-time reader that eight things are equally likely to be
/// what they want. These four headings answer "which one do I type" instead.
struct Group {
    heading: &'static str,
    entries: &'static [Entry],
}

/// The card, in reading order.
const GROUPS: &[Group] = &[
    Group {
        heading: "Start here",
        entries: &[
            Entry {
                name: "init",
                note: "Find your games, ask what you want, apply it.",
            },
            Entry {
                name: "rollback",
                note: "Put everything back. Any run, any time.",
            },
        ],
    },
    Group {
        heading: "Look before you leap",
        entries: &[
            Entry {
                name: "doctor",
                note: "What your machine is, and what each tuning would do.",
            },
            Entry {
                name: "explain",
                note: "One tuning: why it exists, and what it'd change here.",
            },
            Entry {
                name: "list-games",
                note: "The game profiles I can see, and where each came from.",
            },
        ],
    },
    Group {
        heading: "For scripts and repeat runs",
        entries: &[
            Entry {
                name: "apply",
                note: "Apply, optionally one tuning: --step <id>",
            },
            Entry {
                name: "selftest",
                note: "Apply, verify, revert, verify. Proves a tuning works.",
            },
        ],
    },
    Group {
        heading: "Anywhere",
        entries: &[
            Entry {
                name: "--dry-run",
                note: "Work out the plan, change nothing.",
            },
            Entry {
                name: "--yes",
                note: "Take every default, install without asking.",
            },
            Entry {
                name: "--json",
                note: "The run report as JSON instead of this.",
            },
        ],
    },
];

/// What `gameready --help` prints, in place of clap's generated list.
pub struct HelpCard;

impl HelpCard {
    /// The name column, shared by every group so the notes line up down the
    /// whole card rather than per group.
    fn column() -> usize {
        GROUPS
            .iter()
            .flat_map(|group| group.entries)
            .map(|entry| console::measure_text_width(entry.name))
            .max()
            .unwrap_or(0)
    }

    /// The product line: the name, then the version and what it is for.
    fn header() -> String {
        format!(
            "{} {}",
            style("gameready").bold(),
            style(format!("{} · {TAGLINE}", env!("CARGO_PKG_VERSION"))).dim()
        )
    }
}

impl fmt::Display for HelpCard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = Section::new(f);
        let column = Self::column();
        s.title(&Self::header())?;

        for group in GROUPS {
            s.heading(group.heading)?;
            for entry in group.entries {
                s.entry(entry.name, entry.note, column)?;
            }
            s.blank()?;
        }

        s.indented(&style(REVERSIBLE).dim().to_string())?;
        s.end()
    }
}

#[cfg(test)]
#[path = "help_test.rs"]
mod help_test;
