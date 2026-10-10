//! Five complete B♭ Dorian suites with independent phrases and metres.
use super::{
    Movement,
    manuscript::{self, Phrase},
};

/// The five complete B♭ Dorian suites.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
#[repr(u8)]
pub enum Theme {
    /// Glutpfad: spacious fourths and fifths over a warm four-beat tread.
    #[default]
    #[serde(alias = "ballad")]
    Ember = 0,
    /// Mondglas: a lilting three-beat song with answering sixths.
    #[serde(alias = "dance")]
    Glass = 1,
    /// Dornenkrone: dotted calls, terse replies and restless combat strings.
    #[serde(alias = "epic")]
    Thorn = 2,
    /// Nebelhafen: rolling compound metre and an intimate tavern song.
    #[serde(alias = "jig")]
    Tide = 3,
    /// Sternfall: broad 3+2 phrases in five, opening into octave-spanning arcs.
    Star = 4,
}
impl Theme {
    /// Every suite, in settings order.
    pub const ALL: [Self; 5] = [
        Self::Ember,
        Self::Glass,
        Self::Thorn,
        Self::Tide,
        Self::Star,
    ];
    /// Decode the three theme bits; invalid values select the first suite.
    #[must_use]
    pub const fn of(bits: u8) -> Self {
        match bits {
            1 => Self::Glass,
            2 => Self::Thorn,
            3 => Self::Tide,
            4 => Self::Star,
            _ => Self::Ember,
        }
    }
    /// Stable file and development-control name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Ember => "ember",
            Self::Glass => "glass",
            Self::Thorn => "thorn",
            Self::Tide => "tide",
            Self::Star => "star",
        }
    }
    /// Eighth-note pulses: 4/4, 3/4, 4/4, 6/8 or 5/4.
    pub(super) const fn ticks(self) -> u8 {
        match self {
            Self::Ember | Self::Thorn => 8,
            Self::Glass | Self::Tide => 6,
            Self::Star => 10,
        }
    }
    pub(super) fn eighth(self, movement: Movement) -> f64 {
        let quarter = match self {
            Self::Ember => 88.0,
            Self::Glass => 84.0,
            Self::Thorn => 98.0,
            Self::Tide => 92.0,
            Self::Star => 96.0,
        };
        let pace = match movement {
            Movement::Title => 1.0,
            Movement::Lobby => 0.82,
            Movement::Standard => 0.88,
            Movement::Combat => 1.36,
            Movement::Endgame => 1.13,
            Movement::Victory => 1.14,
            Movement::Defeat => 0.68,
            Movement::Draw => 0.81,
        };
        30.0 / (quarter * pace)
    }
    pub(super) const fn pages(self) -> &'static Pages {
        match self {
            Self::Ember => &manuscript::EMBER,
            Self::Glass => &manuscript::GLASS,
            Self::Thorn => &manuscript::THORN,
            Self::Tide => &manuscript::TIDE,
            Self::Star => &manuscript::STAR,
        }
    }
}
/// Independently composed material, in eighth-note durations; 0 is a rest.
pub(super) struct Pages {
    pub title: Phrase,
    pub answer: Phrase,
    pub tavern: Phrase,
    pub table: Phrase,
    pub combat: Phrase,
}
