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

#[test]
fn a_banner_runs_its_rule_out_to_the_layout_width() {
    let mut buf = String::new();
    Section::with_width(&mut buf, ROOMY)
        .banner("STEP 1 OF 4")
        .unwrap();

    assert!(plain(&buf).starts_with("STEP 1 OF 4 -"), "{buf}");
    assert_eq!(console::measure_text_width(plain(&buf).trim_end()), ROOMY);
}

#[test]
fn a_quoted_block_carries_its_bar_down_every_wrapped_line() {
    // A description that wraps is still one description, and a second line
    // starting at the margin would read as a second package.
    let mut buf = String::new();
    let long = "the whole package list and its per-package sizes, with the longest \
                descriptions wrapped to the width";
    Section::with_width(&mut buf, CRAMPED).quoted(long).unwrap();
    let rendered = plain(&buf);

    assert!(rendered.lines().count() > 1, "{rendered}");
    assert!(
        rendered.lines().all(|line| line.starts_with("  ┃ ")),
        "{rendered}"
    );
}

/// Wide enough to hold the longest name these tests use.
const COLUMN: usize = 24;

#[test]
fn every_rows_evidence_starts_at_the_same_column() {
    let mut buf = String::new();
    let mut section = Section::with_width(&mut buf, ROOMY);
    section
        .row(Mark::Applied, "Swappiness", "already 180", COLUMN)
        .unwrap();
    section
        .row(
            Mark::AlreadySet,
            "I/O scheduler nvme0n1",
            "mq-deadline",
            COLUMN,
        )
        .unwrap();
    let rendered = plain(&buf);

    let starts: Vec<usize> = rendered
        .lines()
        .map(|line| line.rfind("  ").expect("a gap before the evidence"))
        .collect();
    assert_eq!(starts[0], starts[1], "{rendered}");
}

#[test]
fn a_long_evidence_wraps_under_itself_rather_than_under_the_mark() {
    let mut buf = String::new();
    let evidence = "wrote 60 and read back 180, which is not what was asked for";
    Section::with_width(&mut buf, CRAMPED)
        .row(Mark::Failed, "Swappiness", evidence, COLUMN)
        .unwrap();
    let rendered = plain(&buf);

    let mut lines = rendered.lines();
    let first = lines.next().expect("a first line");
    let second = lines.next().expect("a wrapped line");
    assert!(first.starts_with("  ✘ Swappiness"), "{rendered}");
    // Wrapped text lands in the evidence column, clear of the name.
    assert!(second.starts_with(&" ".repeat(COLUMN)), "{rendered}");
}

#[test]
fn a_name_wider_than_the_column_pushes_only_its_own_evidence() {
    let mut buf = String::new();
    Section::with_width(&mut buf, ROOMY)
        .row(Mark::Applied, "a step nobody gave a short name", "done", 8)
        .unwrap();
    let rendered = plain(&buf);

    assert!(
        rendered.starts_with("  ✓ a step nobody gave a short name    done"),
        "{rendered}"
    );
}
