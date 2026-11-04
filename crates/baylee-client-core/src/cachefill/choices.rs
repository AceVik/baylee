//! What the player chooses about the fill, kept per device
//! (`ClientSettings::card_fill`).

use crate::i18n::Phrase;
use serde::{Deserialize, Serialize};

/// How many pictures come down at once. Scryfall's image hosts have no rate
/// limit (`docs/legal.md` §3), so this is courtesy and the player's bandwidth
/// and no rate: within it a request starts as soon as the last one is done.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Pace {
    /// One at a time.
    Gentle,
    /// Two at a time.
    #[default]
    Steady,
    /// Three at a time.
    Brisk,
}

impl Pace {
    /// Every pace, as the settings offer them.
    pub const ALL: [Self; 3] = [Self::Gentle, Self::Steady, Self::Brisk];

    /// Its name on the settings' control.
    #[must_use]
    pub const fn name(self) -> Phrase {
        match self {
            Self::Gentle => Phrase::CardFillPaceGentle,
            Self::Steady => Phrase::CardFillPaceSteady,
            Self::Brisk => Phrase::CardFillPaceBrisk,
        }
    }

    /// How many requests this pace has in flight at most.
    #[must_use]
    pub const fn parallel(self) -> usize {
        match self {
            Self::Gentle => 1,
            Self::Steady => 2,
            Self::Brisk => 3,
        }
    }
}

/// How much of every card is kept.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reach {
    /// The size a row and the table draw: about 14 kB a card.
    Thumbnails,
    /// That, then the size a preview draws: about 100 kB more a card.
    #[default]
    Everything,
}

impl Reach {
    /// Every reach, as the settings offer them.
    pub const ALL: [Self; 2] = [Self::Thumbnails, Self::Everything];

    /// Its name on the settings' control.
    #[must_use]
    pub const fn name(self) -> Phrase {
        match self {
            Self::Thumbnails => Phrase::CardFillReachThumbnails,
            Self::Everything => Phrase::CardFillReachEverything,
        }
    }

    /// How many rounds a plan of this reach has.
    #[must_use]
    pub const fn rounds(self) -> u8 {
        match self {
            Self::Thumbnails => 1,
            Self::Everything => 2,
        }
    }
}

/// How large the fill lets the cache grow.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Budget {
    /// One gigabyte: every thumbnail and most of the large pictures.
    One,
    /// Two: the whole pool, and room for what a player looks at.
    #[default]
    Two,
    /// Five.
    Five,
}

impl Budget {
    /// Every budget, as the settings offer them.
    pub const ALL: [Self; 3] = [Self::One, Self::Two, Self::Five];

    /// The size, in bytes as a file manager counts them.
    #[must_use]
    pub const fn bytes(self) -> u64 {
        match self {
            Self::One => 1_000_000_000,
            Self::Two => 2_000_000_000,
            Self::Five => 5_000_000_000,
        }
    }
}

/// Everything the player chooses about the fill.
///
/// On by default on a desktop: the owner asked for it, a picture costs the
/// player nothing a draw would not cost, the fill rests during a game, and it
/// stops at [`Budget`] and [`super::FREE_FLOOR`]. The switch is the player's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct FillChoices {
    /// Whether it runs at all.
    pub on: bool,
    /// How fast.
    pub pace: Pace,
    /// How much of a card.
    pub reach: Reach,
    /// Up to what size.
    pub budget: Budget,
}

impl Default for FillChoices {
    fn default() -> Self {
        Self {
            on: true,
            pace: Pace::default(),
            reach: Reach::default(),
            budget: Budget::default(),
        }
    }
}
