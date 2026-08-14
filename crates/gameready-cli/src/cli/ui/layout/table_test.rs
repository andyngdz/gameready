use super::super::Section;
use super::*;

/// Wide enough to hold the longest name these tests use.
const COLUMN: usize = 24;

fn plain(text: &str) -> String {
    console::strip_ansi_codes(text).into_owned()
}

fn rendered(rows: &[(Mark, &str, &str)]) -> String {
    let mut table = ResultTable::new(COLUMN);
    for (mark, name, evidence) in rows {
        table.row(*mark, name, evidence);
    }
    plain(&table.to_string())
}

/// Where in a rendered line the given evidence begins, in columns.
fn starts_at(text: &str, evidence: &str) -> usize {
    let line = text
        .lines()
        .find(|line| line.contains(evidence))
        .expect("a row carrying that evidence");
    console::measure_text_width(line.split(evidence).next().expect("what comes first"))
}

#[test]
fn every_rows_evidence_starts_at_the_same_column() {
    let text = rendered(&[
        (Mark::AlreadySet, "Swappiness", "already 180"),
        (Mark::Applied, "I/O scheduler nvme0n1", "mq-deadline"),
    ]);

    assert_eq!(
        starts_at(&text, "already 180"),
        starts_at(&text, "mq-deadline"),
        "{text}"
    );
}

#[test]
fn a_short_name_does_not_pull_the_column_in() {
    // The column is pinned, so a table of short names lines up with a table of
    // long ones on the screen before it.
    let short = rendered(&[(Mark::Applied, "Swappiness", "already 180")]);
    let long = rendered(&[(Mark::Applied, "I/O scheduler nvme0n1", "mq-deadline")]);

    assert_eq!(
        starts_at(&short, "already 180"),
        starts_at(&long, "mq-deadline"),
        "{short}{long}"
    );
}

#[test]
fn long_evidence_wraps_inside_its_own_column() {
    let text = rendered(&[(
        Mark::Skipped,
        "Swappiness",
        "swap is on disk, not zram, so the default swappiness is already the right one for this \
         machine and nothing here would improve it",
    )]);

    assert!(text.lines().count() > 1, "{text}");
    for line in text.lines() {
        assert!(
            console::measure_text_width(line) <= crate::cli::ui::layout::width(),
            "over the layout width: {line}"
        );
    }
}

#[test]
fn a_table_row_and_a_section_row_land_on_the_same_column() {
    // The summary interleaves rows with the blocks a failure gets, so it draws
    // its rows one at a time through Section rather than as one table. The two
    // sit on the same screen as each other and on screens either side, so they
    // have to agree about where the evidence starts.
    let table = rendered(&[(Mark::Applied, "Swappiness", "already 180")]);

    let mut section_out = String::new();
    Section::new(&mut section_out)
        .row(Mark::Applied, "Swappiness", "already 180", COLUMN)
        .expect("writing into a string");
    let section = plain(&section_out);

    assert_eq!(
        starts_at(&table, "already 180"),
        starts_at(&section, "already 180"),
        "table={table}\nsection={section}"
    );
}

#[test]
fn every_line_carries_the_layout_indent() {
    let text = rendered(&[(Mark::Applied, "Swappiness", "already 180")]);

    for line in text.lines() {
        assert!(line.starts_with(INDENT), "{text}");
    }
}
