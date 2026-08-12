//! Asking whether this run points the games at the newest Proton-GE.

use std::fmt;

use anyhow::Result;
use gameready_core::steps::CompatWish;

use crate::cli::ui::{games_noun, theme};

/// The question.
const QUESTION: &str = "Use the newest Proton-GE for your games?";

/// The keys, in the order a user reaches for them.
const KEYS: &str = "up down move · enter confirm · esc keeps the default";

/// What the user wants done about the Proton build their games run on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtonPin {
    /// Point the picked games at it, and make it the default for the rest.
    UseNewest,

    /// Leave every Proton setting exactly as Steam has it.
    KeepCurrent,
}

impl ProtonPin {
    /// The entries this answer writes, given what the profiles asked for.
    ///
    /// The machine-wide default rides along with the games rather than being a
    /// second question. Installing a build changes nothing on its own, so a
    /// user saying yes here is saying they want it used, and a game with no
    /// profile is still their game.
    #[must_use]
    pub fn wishes(self, games: Vec<CompatWish>) -> Vec<CompatWish> {
        match self {
            Self::KeepCurrent => Vec::new(),
            Self::UseNewest => {
                let mut wishes = games;
                wishes.push(CompatWish::machine_wide());
                wishes
            }
        }
    }

    /// Why installing a build is not the same as running on it.
    fn what_changes(games: usize) -> String {
        let noun = games_noun(games);
        format!(
            "Steam keeps running whatever build it already had, so installing one changes nothing \
             on its own. This picks it for {games} {noun} and makes it the default for everything \
             else. Rollback puts your old config back exactly."
        )
    }
}

impl fmt::Display for ProtonPin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::UseNewest => "Yes, use it everywhere",
            Self::KeepCurrent => "No, leave my Proton settings alone",
        })
    }
}

/// Asks whether the newest Proton-GE becomes what the games run on.
///
/// Defaults to leaving Steam alone, and an escaped prompt leaves it alone. The
/// Proton build is the setting a user is most likely to have picked for
/// themselves, so overwriting it is never where an interrupted prompt lands.
pub fn choose_proton_pin(games: usize) -> Result<ProtonPin> {
    let answer = theme::Asked::new(QUESTION, &ProtonPin::what_changes(games), KEYS)
        .one_of(vec![ProtonPin::KeepCurrent, ProtonPin::UseNewest])
        .prompt_skippable()?;

    Ok(answer.unwrap_or(ProtonPin::KeepCurrent))
}

#[cfg(test)]
#[path = "proton_choice_test.rs"]
mod proton_choice_test;
