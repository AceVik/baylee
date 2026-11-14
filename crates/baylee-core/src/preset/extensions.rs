//! Bounded references to a compiled rule pack, never executable rules or scripts.
use serde::{Deserialize, Serialize};

/// Structural ceiling; an individual pack may impose a smaller equipment limit.
pub const MAX_EXTENSIONS_PER_SEAT: usize = 8;

/// Frozen native pack and its per-seat definition references.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RulePackSetup {
    /// Nonzero, stable pack namespace. A compiled host must explicitly admit it.
    pub pack: u16,
    /// Exact definition revision; never negotiated or substituted during setup.
    pub revision: u16,
    /// Canonical ascending seat order, with at most one entry per seat.
    pub seats: Vec<SeatExtensions>,
}

/// Active definitions, in slot order. Reserve inventory never enters a duel.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeatExtensions {
    /// Zero-based seat.
    pub seat: u8,
    /// Nonzero immutable definition IDs. Repeated IDs mean distinct equipped copies.
    pub definitions: Vec<u16>,
}

impl RulePackSetup {
    pub(super) fn valid(&self, seats: usize, custom: bool) -> bool {
        custom
            && self.pack != 0
            && self.revision != 0
            && self.seats.len() <= seats
            && self.seats.iter().enumerate().all(|(index, entry)| {
                usize::from(entry.seat) < seats
                    && (index == 0 || self.seats[index - 1].seat < entry.seat)
                    && entry.definitions.len() <= MAX_EXTENSIONS_PER_SEAT
                    && entry.definitions.iter().all(|definition| *definition != 0)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> RulePackSetup {
        RulePackSetup {
            pack: 1,
            revision: 2,
            seats: vec![SeatExtensions {
                seat: 0,
                definitions: vec![3, 3, 7],
            }],
        }
    }

    #[test]
    fn native_references_are_bounded_canonical_and_custom_only() {
        let good = setup();
        assert!(good.valid(2, true));
        assert!(!good.valid(2, false));
        assert!(
            !RulePackSetup {
                pack: 0,
                ..good.clone()
            }
            .valid(2, true)
        );
        assert!(
            !RulePackSetup {
                revision: 0,
                ..good.clone()
            }
            .valid(2, true)
        );
        for seats in [
            vec![SeatExtensions {
                seat: 2,
                definitions: vec![],
            }],
            vec![good.seats[0].clone(), good.seats[0].clone()],
            vec![
                SeatExtensions {
                    seat: 1,
                    definitions: vec![],
                },
                good.seats[0].clone(),
            ],
            vec![SeatExtensions {
                seat: 0,
                definitions: vec![0],
            }],
            vec![SeatExtensions {
                seat: 0,
                definitions: vec![1; MAX_EXTENSIONS_PER_SEAT + 1],
            }],
        ] {
            assert!(
                !RulePackSetup {
                    seats,
                    ..good.clone()
                }
                .valid(2, true)
            );
        }
    }

    #[test]
    fn native_references_cannot_carry_commands_or_parameter_overrides() {
        let mut value = serde_json::to_value(setup()).unwrap();
        value["script"] = "grant_life(999)".into();
        assert!(serde_json::from_value::<RulePackSetup>(value).is_err());
        let mut value = serde_json::to_value(setup()).unwrap();
        value["seats"][0]["life"] = 999.into();
        assert!(serde_json::from_value::<RulePackSetup>(value).is_err());
    }
}
