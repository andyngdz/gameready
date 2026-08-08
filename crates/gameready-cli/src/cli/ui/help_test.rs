use clap::CommandFactory;

use super::HelpCard;
use crate::cli::args::Cli;

fn card() -> String {
    console::strip_ansi_codes(&HelpCard.to_string()).into_owned()
}

/// The line the card gives `name`, without its leading indent.
fn line(rendered: &str, name: &str) -> String {
    rendered
        .lines()
        .find(|line| line.trim_start().starts_with(name))
        .unwrap_or_else(|| panic!("no line for {name} in\n{rendered}"))
        .to_owned()
}

#[test]
fn the_header_names_the_version_this_binary_is() {
    assert!(card().contains(env!("CARGO_PKG_VERSION")), "{}", card());
}

#[test]
fn every_subcommand_is_on_the_card() {
    // The card is written by hand, so a subcommand added later is invisible
    // until someone adds it here. This is the someone.
    let rendered = card();
    for command in Cli::command().get_subcommands() {
        let name = command.get_name();
        if name == "help" {
            continue;
        }
        assert!(
            rendered.contains(name),
            "{name} is missing from\n{rendered}"
        );
    }
}

#[test]
fn the_notes_line_up_across_groups() {
    // The name column is shared by every group, so the eye runs down one edge
    // rather than down four.
    let rendered = card();
    let note = |name: &str, note: &str| {
        line(&rendered, name)
            .find(note)
            .unwrap_or_else(|| panic!("{name} does not say {note}"))
    };

    assert_eq!(
        note("init", "Find your games"),
        note("--json", "The run report"),
    );
}

#[test]
fn each_group_keeps_its_heading() {
    let rendered = card();
    for heading in [
        "Start here",
        "Look before you leap",
        "For scripts and repeat runs",
        "Anywhere",
    ] {
        assert!(rendered.contains(heading), "{heading} is missing");
    }
}

#[test]
fn the_card_says_which_change_cannot_be_taken_back() {
    assert!(card().contains("except installing a package"), "{}", card());
}
