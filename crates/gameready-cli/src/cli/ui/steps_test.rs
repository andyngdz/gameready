use super::Steps;

fn header(steps: &mut Steps, caution: Option<&str>) -> String {
    steps.asked = steps.asked.saturating_add(1);
    console::strip_ansi_codes(&steps.rendered(caution)).into_owned()
}

#[test]
fn each_question_is_numbered_against_the_total() {
    let mut steps = Steps::of(4);

    assert!(header(&mut steps, None).contains("STEP 1 OF 4"));
    assert!(header(&mut steps, None).contains("STEP 2 OF 4"));
}

#[test]
fn the_caution_rides_in_the_header_rather_than_under_the_question() {
    let mut steps = Steps::of(4);
    let rendered = header(&mut steps, Some("the one thing rollback can't undo"));

    assert!(
        rendered.contains("STEP 1 OF 4 · THE ONE THING ROLLBACK CAN'T UNDO"),
        "{rendered}"
    );
}

#[test]
fn the_header_rule_runs_to_the_layout_width() {
    // The rule is what separates one question from the answered one above it,
    // so it has to reach the edge rather than stop where the label does.
    let mut steps = Steps::of(4);
    let rendered = header(&mut steps, None);
    let line = rendered
        .lines()
        .find(|line| !line.is_empty())
        .expect("line");

    assert_eq!(
        console::measure_text_width(line),
        crate::cli::ui::layout::width()
    );
}
