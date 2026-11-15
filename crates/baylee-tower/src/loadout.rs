//! One mapping from validated run inventory to frozen engine setup resources.

use crate::{Refusal, economy::Economy, items};
use baylee_core::preset::{SeatOpening, SeatSupplies};

/// Resource portion of a duel package. The authority also fixes the legal deck,
/// opponent, rules version, shuffle and policy seeds before handing it to a host.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DuelResources {
    /// Applied to `SeatSpec::starting_life`.
    pub starting_life: i32,
    /// Applied to the matching seat's house opening rules.
    pub opening: SeatOpening,
    /// Applied to the matching seat's custom-mode supplies.
    pub supplies: SeatSupplies,
}

/// Convert authoritative inventory without trusting client-supplied attributes.
///
/// # Errors
/// A seat outside a duel, malformed economy, or invalid equipment/charge bounds.
pub fn resources(economy: &Economy, seat: u8) -> Result<DuelResources, Refusal> {
    if seat > 1 {
        return Err(Refusal::Invalid);
    }
    economy.validate()?;
    let attributes = items::attributes(&economy.equipment)?;
    Ok(DuelResources {
        starting_life: i32::from(attributes.life),
        opening: SeatOpening {
            seat,
            cards: attributes.opening_cards,
            maximum_hand: attributes.maximum_hand,
            free_mulligans: attributes.free_mulligans,
        },
        supplies: SeatSupplies {
            seat,
            healing: economy.potions[0],
            mana: economy.potions[1],
            seedling: attributes.seedlings == 1,
            concealed: false,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::items::{Equipment, Relic};

    #[test]
    fn tower_loadout_preserves_each_bought_resource_for_the_correct_seat() {
        let mut economy = Economy {
            equipment: vec![
                Equipment {
                    relic: Relic::Heartwood,
                    rank: 3,
                },
                Equipment {
                    relic: Relic::Satchel,
                    rank: 1,
                },
                Equipment {
                    relic: Relic::Compass,
                    rank: 2,
                },
            ],
            potions: [1, 1],
            ..Economy::default()
        };
        let duel = resources(&economy, 1).unwrap();
        assert_eq!(duel.starting_life, 32);
        assert_eq!(
            duel.opening,
            SeatOpening {
                seat: 1,
                cards: 8,
                maximum_hand: 8,
                free_mulligans: 3
            }
        );
        assert_eq!(
            duel.supplies,
            SeatSupplies {
                seat: 1,
                healing: 1,
                mana: 1,
                seedling: false,
                concealed: false
            }
        );
        economy.equipment[0] = Equipment {
            relic: Relic::Seedling,
            rank: 1,
        };
        let duel = resources(&economy, 0).unwrap();
        assert_eq!(duel.starting_life, 20);
        assert!(duel.supplies.seedling);
        assert_eq!(duel.opening.seat, 0);
        assert_eq!(duel.supplies.seat, 0);
        assert_eq!(resources(&economy, 2), Err(Refusal::Invalid));
        economy.potions = [255, 255];
        assert_eq!(resources(&economy, 0), Err(Refusal::Invalid));
    }
}
