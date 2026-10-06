use serde::{Deserialize, Serialize};
use std::borrow::Cow;

// ------------------------------------------------------------------ counters

/// A counter kind, wire-stable.
///
/// The engine's counter enum carries a `Custom(u32)` payload; it is preserved
/// here so a client can render an unknown counter by name rather than dropping
/// it.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum CounterKind {
    /// A +X/+Y counter (CR 122.1a).
    Plus {
        /// X.
        power: u8,
        /// Y.
        toughness: u8,
    },
    /// A -X/-Y counter (CR 122.1a).
    Minus {
        /// X.
        power: u8,
        /// Y.
        toughness: u8,
    },
    /// Loyalty counter.
    Loyalty,
    /// Lore counter (sagas).
    Lore,
    /// Time counter (suspend, vanishing).
    Time,
    /// Charge counter.
    Charge,
    /// Poison counter.
    Poison,
    /// Energy counter.
    Energy,
    /// Rad counter.
    Rad,
    /// Lifelink keyword counter.
    Lifelink,
    /// Level counter.
    Level,
    /// Any other counter, identified by the engine's opaque id.
    Custom(u32),
}

impl CounterKind {
    /// The +1/+1 counter, as one value rather than a second spelling of it.
    pub const PLUS_ONE: Self = Self::Plus {
        power: 1,
        toughness: 1,
    };
    /// The -1/-1 counter.
    pub const MINUS_ONE: Self = Self::Minus {
        power: 1,
        toughness: 1,
    };

    /// Whether the counter changes power/toughness, which a client renders on
    /// the card face rather than as a badge.
    #[must_use]
    pub const fn is_power_toughness(self) -> bool {
        matches!(self, Self::Plus { .. } | Self::Minus { .. })
    }

    /// A short badge label.
    ///
    /// Borrowed for every counter whose name is a word and owned for a P/T
    /// counter, whose name is its two numbers — a `-0/-1` badge cannot be a
    /// `&'static str` because the pair is open-ended (CR 122.1a), and the
    /// two common ones are still handed back without allocating.
    #[must_use]
    pub fn badge(self) -> Cow<'static, str> {
        match self {
            Self::PLUS_ONE => Cow::Borrowed("+1/+1"),
            Self::MINUS_ONE => Cow::Borrowed("-1/-1"),
            Self::Plus { power, toughness } => Cow::Owned(format!("+{power}/+{toughness}")),
            Self::Minus { power, toughness } => Cow::Owned(format!("-{power}/-{toughness}")),
            Self::Loyalty => Cow::Borrowed("LOY"),
            Self::Lore => Cow::Borrowed("LORE"),
            Self::Time => Cow::Borrowed("TIME"),
            Self::Charge => Cow::Borrowed("CHG"),
            Self::Poison => Cow::Borrowed("PSN"),
            Self::Energy => Cow::Borrowed("NRG"),
            Self::Rad => Cow::Borrowed("RAD"),
            Self::Lifelink => Cow::Borrowed("LL"),
            Self::Level => Cow::Borrowed("LVL"),
            Self::Custom(_) => Cow::Borrowed("•"),
        }
    }
}

/// A counter kind together with how many are on the object.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct CounterEntry {
    /// Which counter.
    pub kind: CounterKind,
    /// How many.
    pub count: u16,
}
