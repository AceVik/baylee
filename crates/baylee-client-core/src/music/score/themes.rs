//! The five Dorian suites and ten additional independent musical styles.
use super::{
    Movement,
    manuscript::{self, Phrase},
};

/// Fifteen complete suites, with eight movements each.
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
    /// Velvet Night · Piano.
    Velvet = 5,
    /// Copperwork · Baroque.
    Copper = 6,
    /// Juniper · Folk.
    Juniper = 7,
    /// Lagoon Light · Bossa.
    Lagoon = 8,
    /// Lanterns · Jazz waltz.
    Lantern = 9,
    /// Neon Path · Synthwave.
    Neon = 10,
    /// Pixelstorm · Chiptune.
    Circuit = 11,
    /// Mosaic · Marimba.
    Mosaic = 12,
    /// Orbit · Ambient.
    Orbit = 13,
    /// Iron Pulse · Breakbeat.
    Iron = 14,
}
impl Theme {
    /// The original five suites, retained without musical changes.
    pub const DORIAN: [Self; 5] = [
        Self::Ember,
        Self::Glass,
        Self::Thorn,
        Self::Tide,
        Self::Star,
    ];
    /// Ten additional independent styles.
    pub const EXPLORATIONS: [Self; 10] = [
        Self::Velvet,
        Self::Copper,
        Self::Juniper,
        Self::Lagoon,
        Self::Lantern,
        Self::Neon,
        Self::Circuit,
        Self::Mosaic,
        Self::Orbit,
        Self::Iron,
    ];
    /// Whether this is one of the ten additional styles.
    #[must_use]
    pub const fn exploration(self) -> bool {
        self as u8 >= 5
    }
    /// Every suite, in settings order.
    pub const ALL: [Self; 15] = [
        Self::Ember,
        Self::Glass,
        Self::Thorn,
        Self::Tide,
        Self::Star,
        Self::Velvet,
        Self::Copper,
        Self::Juniper,
        Self::Lagoon,
        Self::Lantern,
        Self::Neon,
        Self::Circuit,
        Self::Mosaic,
        Self::Orbit,
        Self::Iron,
    ];
    /// Decode the four theme bits; invalid values select the first suite.
    #[must_use]
    pub const fn of(bits: u8) -> Self {
        match bits {
            1 => Self::Glass,
            2 => Self::Thorn,
            3 => Self::Tide,
            4 => Self::Star,
            5 => Self::Velvet,
            6 => Self::Copper,
            7 => Self::Juniper,
            8 => Self::Lagoon,
            9 => Self::Lantern,
            10 => Self::Neon,
            11 => Self::Circuit,
            12 => Self::Mosaic,
            13 => Self::Orbit,
            14 => Self::Iron,

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
            Self::Velvet => "velvet",
            Self::Copper => "copper",
            Self::Juniper => "juniper",
            Self::Lagoon => "lagoon",
            Self::Lantern => "lantern",
            Self::Neon => "neon",
            Self::Circuit => "circuit",
            Self::Mosaic => "mosaic",
            Self::Orbit => "orbit",
            Self::Iron => "iron",
        }
    }
    /// Eighth-note pulses: 4/4, 3/4, 4/4, 6/8 or 5/4.
    pub(in crate::music) const fn ticks(self) -> u8 {
        match self {
            Self::Ember | Self::Thorn => 8,
            Self::Glass | Self::Tide => 6,
            Self::Star => 10,
            other => super::styles::profile(other).ticks,
        }
    }
    pub(super) fn eighth(self, movement: Movement) -> f64 {
        let quarter = match self {
            Self::Ember => 112.0,
            Self::Glass => 132.0,
            Self::Thorn => 124.0,
            Self::Tide => 138.0,
            Self::Star => 120.0,
            other => super::styles::profile(other).bpm,
        };
        let pace = match movement {
            Movement::Title => 1.0,
            Movement::Lobby => 0.88,
            Movement::Standard => 0.96,
            Movement::Combat => 1.18,
            Movement::Endgame => 1.13,
            Movement::Victory => 1.14,
            Movement::Defeat => 0.78,
            Movement::Draw => 0.90,
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
            other => super::styles::profile(other).pages,
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
