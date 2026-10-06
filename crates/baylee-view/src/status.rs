use serde::{Deserialize, Serialize};

// -------------------------------------------------------------------- status

/// Public status bits of a permanent (CR 110.5).
///
/// A newtype rather than a bare integer so a client cannot accidentally read a
/// bit that does not exist. Mirrors the engine's `Status`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ObjectStatus(u8);

impl ObjectStatus {
    /// No status bits set.
    pub const NONE: Self = Self(0);
    /// Tapped.
    pub const TAPPED: Self = Self(1);
    /// Face down (morph, manifest, …).
    pub const FACE_DOWN: Self = Self(2);
    /// Phased out (CR 702.26).
    pub const PHASED_OUT: Self = Self(4);
    /// Flipped (flip cards).
    pub const FLIPPED: Self = Self(8);

    /// Builds a status from the engine's raw bits.
    #[must_use]
    pub const fn from_bits(bits: u8) -> Self {
        Self(bits)
    }

    /// The raw bits.
    #[must_use]
    pub const fn bits(self) -> u8 {
        self.0
    }

    /// Whether every bit of `other` is set.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Whether the permanent is tapped.
    #[must_use]
    pub const fn is_tapped(self) -> bool {
        self.contains(Self::TAPPED)
    }

    /// Whether the permanent is face down.
    #[must_use]
    pub const fn is_face_down(self) -> bool {
        self.contains(Self::FACE_DOWN)
    }

    /// Whether the permanent is phased out — clients render these ghosted and
    /// exclude them from board summaries.
    #[must_use]
    pub const fn is_phased_out(self) -> bool {
        self.contains(Self::PHASED_OUT)
    }
}
