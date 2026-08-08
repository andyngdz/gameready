use super::*;

/// A width narrow enough that a long row cannot fit its leader.
const CRAMPED: usize = 60;

/// A width wide enough for a leader and some room either side.
const ROOMY: usize = 80;

fn plain(text: &str) -> String {
    console::strip_ansi_codes(text).into_owned()
}

#[test]
fn section_title_adds_a_blank_line() {
    let mut buf = String::new();
    Section::new(&mut buf).title("Test:").unwrap();
    assert_eq!(buf, "Test:\n\n");
}

#[test]
fn marked_line_is_indented_with_its_mark() {
    let mut buf = String::new();
    Section::new(&mut buf)
        .marked(Mark::Chosen, "hello")
        .unwrap();
    assert_eq!(plain(&buf), "  * hello\n");
}

#[test]
fn indented_line_has_no_mark() {
    let mut buf = String::new();
    Section::new(&mut buf).indented("text").unwrap();
    assert_eq!(buf, "  text\n");
}

#[test]
fn the_separator_is_as_wide_as_the_layout() {
    let mut buf = String::new();
    Section::with_width(&mut buf, ROOMY).end().unwrap();
    assert_eq!(plain(buf.trim_end()).len(), ROOMY);
}

#[test]
fn a_row_fills_the_width_so_the_evidence_lands_on_the_right_edge() {
    let mut buf = String::new();
    Section::with_width(&mut buf, ROOMY)
        .row(
            Mark::Applied,
            "Raise vm.max_map_count",
            Some("65530 to 2147483642"),
        )
        .unwrap();

    let line = plain(buf.trim_end());
    assert_eq!(line.chars().count(), ROOMY);
    assert!(line.ends_with(". 65530 to 2147483642"), "{line}");
}

#[test]
fn a_row_with_no_evidence_is_just_a_marked_line() {
    let mut buf = String::new();
    Section::with_width(&mut buf, ROOMY)
        .row(Mark::Applied, "Competing daemons", None)
        .unwrap();
    assert_eq!(plain(&buf).trim_end(), "  \u{2713} Competing daemons");
}

#[test]
fn evidence_with_no_room_for_a_leader_drops_to_its_own_line() {
    let mut buf = String::new();
    Section::with_width(&mut buf, CRAMPED)
        .row(
            Mark::Applied,
            "I/O scheduler for the boot disk nvme0n1",
            Some("mq-deadline to none"),
        )
        .unwrap();

    let rendered = plain(&buf);
    let lines: Vec<&str> = rendered.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(!lines[0].contains('.'), "{}", lines[0]);
    assert_eq!(lines[1], "     mq-deadline to none");
}

#[test]
fn a_labelled_paragraph_wraps_inside_the_layout_width() {
    let mut buf = String::new();
    let prose = "Steam rewrites its own config when it quits, so it has to close \
                 before anything writes launch options into it.";
    Section::with_width(&mut buf, CRAMPED)
        .labelled("Why", prose)
        .unwrap();

    let rendered = plain(&buf);
    assert!(rendered.lines().count() > 1);
    for line in rendered.lines() {
        assert!(line.chars().count() <= CRAMPED, "too long: {line}");
    }
}

#[test]
fn only_the_first_line_of_a_labelled_paragraph_carries_the_label() {
    let mut buf = String::new();
    Section::with_width(&mut buf, CRAMPED)
        .labelled("Why", &"word ".repeat(30))
        .unwrap();

    let rendered = plain(&buf);
    let labelled: Vec<&str> = rendered
        .lines()
        .filter(|line| line.contains("Why"))
        .collect();
    assert_eq!(labelled.len(), 1);
}

#[test]
fn a_wrapped_line_hangs_clear_of_the_gutter() {
    // Otherwise the remainder of a long step name lines up with the marks and
    // reads as a step of its own.
    let mut buf = String::new();
    Section::with_width(&mut buf, CRAMPED)
        .marked(Mark::Applied, &"word ".repeat(30))
        .unwrap();

    let rendered = plain(&buf);
    for line in rendered.lines().skip(1) {
        assert!(line.starts_with("    word"), "{line}");
    }
}
