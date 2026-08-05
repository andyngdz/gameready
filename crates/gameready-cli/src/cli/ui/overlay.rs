//! Asking whether to put the frame-rate overlay on screen.

use anyhow::Result;
use gameready_core::steam::Overlay;
use inquire::Confirm;

/// Asks whether the games being set up should show a frame-rate overlay.
///
/// Defaults to no, and an escaped prompt answers no. The overlay covers a
/// corner of the screen with load, temperatures, and a frametime graph, so the
/// safe answer is the one that changes nothing about what the user sees.
pub fn choose_overlay() -> Result<Overlay> {
    let answer = Confirm::new("Show a frame-rate overlay while you play?")
        .with_default(false)
        .with_help_message(
            "mangohud draws FPS and temperatures over the game; you can change this later",
        )
        .prompt_skippable()?;

    Ok(if answer == Some(true) {
        Overlay::Show
    } else {
        Overlay::Hide
    })
}

#[cfg(test)]
#[path = "overlay_test.rs"]
mod overlay_test;
