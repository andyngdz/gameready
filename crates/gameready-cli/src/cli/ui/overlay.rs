//! Asking whether to put the frame-rate overlay on screen.

use std::fmt;

use anyhow::Result;
use gameready_core::steam::Overlay;
use inquire::Select;

use crate::cli::ui::theme;

/// The question.
const QUESTION: &str = "Want a frame-rate overlay while you play?";

/// What the overlay is, and what it is for. The last sentence is the one that
/// matters: nothing here is a decision the user is stuck with.
const WHAT_IT_IS: &str = "MangoHud draws FPS and temperatures over the game. It is how you check \
                          whether any of this helped. Toggle it later any time.";

/// The keys, in the order a user reaches for them.
const KEYS: &str = "↑↓ move · enter confirm · esc keeps the default";

/// Display wrapper so the Select prompt can render `Overlay` values.
struct OverlayOption(Overlay);

impl fmt::Display for OverlayOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self.0 {
            Overlay::Show => "Yes, show FPS and temperatures",
            Overlay::Hide => "No, keep my screen clean",
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
        &theme::asked(QUESTION, WHAT_IT_IS),
        vec![OverlayOption(Overlay::Hide), OverlayOption(Overlay::Show)],
    )
    .with_render_config(theme::questions())
    .with_help_message(KEYS)
    .prompt_skippable()?;

    Ok(answer.map_or(Overlay::Hide, |opt| opt.0))
}

#[cfg(test)]
#[path = "overlay_test.rs"]
mod overlay_test;
