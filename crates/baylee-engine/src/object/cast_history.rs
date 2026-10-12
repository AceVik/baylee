//! One-byte casting provenance; copies do not inherit the actual caster.
use baylee_core::ids::PlayerId;

/// Hand-origin flag and actual caster without increasing every object's footprint.
/// The high bit stores origin; the low seven store caster + 1, with zero meaning
/// uncast. Presets currently allow eight seats; this encoding accommodates 127.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CastHistory(u8);

impl Default for CastHistory {
    fn default() -> Self {
        Self(0x80)
    }
}

impl CastHistory {
    /// Whether the casting origin was the hand (used by rebound).
    #[must_use]
    pub const fn was_from_hand(self) -> bool {
        self.0 & 0x80 != 0
    }

    /// Record casting origin without changing the actual caster.
    pub fn set_from_hand(&mut self, from_hand: bool) {
        self.0 = (self.0 & 0x7f) | (u8::from(from_hand) << 7);
    }

    /// Actual caster, absent on preset permanents, noncast entries and spell copies.
    #[must_use]
    pub const fn caster(self) -> Option<PlayerId> {
        match self.0 & 0x7f {
            0 => None,
            n => Some(PlayerId::new(n - 1)),
        }
    }

    /// Record or clear the caster while retaining the casting origin flag.
    ///
    /// # Panics
    /// Panics for seat IDs of 127 or greater, outside the supported encoding.
    pub fn set_caster(&mut self, caster: Option<PlayerId>) {
        let encoded = caster.map_or(0, |player| {
            assert!(
                player.get() < 127,
                "caster exceeds the supported seat range"
            );
            player.get() + 1
        });
        self.0 = (self.0 & 0x80) | encoded;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_casting_encodings_keep_origin_and_caster_independent() {
        assert_eq!(size_of::<CastHistory>(), 1);
        for hand in [false, true] {
            for seat in 0..127 {
                let mut history = CastHistory::default();
                history.set_from_hand(hand);
                history.set_caster(Some(PlayerId::new(seat)));
                assert_eq!(history.caster(), Some(PlayerId::new(seat)));
                assert_eq!(history.was_from_hand(), hand);
                history.set_from_hand(!hand);
                assert_eq!(history.caster(), Some(PlayerId::new(seat)));
                history.set_caster(None);
                assert_eq!(history.caster(), None);
                assert_eq!(history.was_from_hand(), !hand);
            }
        }
    }
    #[test]
    #[should_panic(expected = "caster exceeds")]
    fn an_invalid_seat_cannot_alias_an_uncast_object() {
        CastHistory::default().set_caster(Some(PlayerId::new(127)));
    }
}
