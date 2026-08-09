use super::*;

/// The PNG signature every encoded dot must start with.
const PNG_MAGIC: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

#[test]
fn the_shipped_artwork_renders_at_every_size_a_panel_can_ask_for() {
    let icons = controller(Ink::Light).expect("the shipped artwork should render");

    let widths: Vec<i32> = icons.iter().map(|icon| icon.width).collect();
    let expected: Vec<i32> = SIZES.iter().map(|&size| size as i32).collect();
    assert_eq!(widths, expected);
}

#[test]
fn every_rendered_icon_carries_four_bytes_a_pixel() {
    let icons = controller(Ink::Light).expect("the shipped artwork should render");

    for icon in &icons {
        let pixels = (icon.width * icon.height) as usize;
        assert_eq!(
            icon.data.len(),
            pixels * 4,
            "{}x{}",
            icon.width,
            icon.height
        );
    }
}

#[test]
fn the_artwork_is_actually_drawn_rather_than_rendering_blank() {
    let icons = controller(Ink::Light).expect("the shipped artwork should render");
    let smallest = icons.first().expect("at least one size");

    // Every fourth byte is alpha. A fully transparent render means the SVG
    // parsed and then landed outside the pixmap, which looks like a missing
    // icon rather than a failure.
    let opaque = smallest
        .data
        .chunks_exact(4)
        .filter(|argb| argb[0] > 0)
        .count();
    assert!(opaque > 0, "nothing was drawn");
}

#[test]
fn the_same_artwork_comes_back_in_whatever_colour_it_was_asked_for() {
    let light = controller(Ink::Light).expect("light should render");
    let live = controller(Ink::Live).expect("live should render");

    let light_first = light.first().expect("at least one size");
    let live_first = live.first().expect("at least one size");

    // Same shape, different pixels: the coverage is identical and only the
    // colour underneath it moved.
    let alpha =
        |icon: &ksni::Icon| -> Vec<u8> { icon.data.chunks_exact(4).map(|argb| argb[0]).collect() };
    assert_eq!(alpha(light_first), alpha(live_first));
    assert_ne!(light_first.data, live_first.data);
}

#[test]
fn a_status_dot_is_a_png_a_menu_host_can_read() {
    let bytes = dot(Ink::Live).expect("a dot should encode");

    assert_eq!(bytes.get(..PNG_MAGIC.len()), Some(&PNG_MAGIC[..]));
}

#[test]
fn each_status_gets_its_own_dot() {
    let live = dot(Ink::Live).expect("live should encode");
    let muted = dot(Ink::Muted).expect("muted should encode");

    assert_ne!(live, muted);
}
