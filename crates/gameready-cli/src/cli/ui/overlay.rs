//! Asking whether to put the frame-rate overlay on screen.

use std::fmt;

use anyhow::Result;
use gameready_core::steam::Overlay;
use inquire::Select;

/// Display wrapper so the Select prompt can render `Overlay` values.
struct OverlayOption(Overlay);

impl fmt::Display for OverlayOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self.0 {
            Overlay::Show => "Yes, show FPS and temperatures while I play",
            Overlay::Hide => "No, keep the screen clean",
        })
    }
}

/// Asks whether the games being set up should show a frame-rate overlay.
///
/// Defaults to hiding it, and an escaped prompt hides it. The overlay covers a
/// corner of the screen with load, temperatures, and a frametime graph, so the
/// safe answer is the one that changes nothing about what the user sees.
pub fn choose_overlay() -> Result<Overlay> {
    let answer = Select::new(
        "Show a frame-rate overlay while you play?",
        vec![OverlayOption(Overlay::Hide), OverlayOption(Overlay::Show)],
    )
    .with_help_message(
        "mangohud draws FPS and temperatures over the game; you can change this later",
    )
    .prompt_skippable()?;

    Ok(answer.map_or(Overlay::Hide, |opt| opt.0))
}

#[cfg(test)]
#[path = "overlay_test.rs"]
mod overlay_test;
