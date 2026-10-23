//! Jinx — {1}{U} — Instant
//! Oracle: Target land becomes the basic land type of your choice until end of turn.
//! Oracle: Draw a card at the beginning of the next turn's upkeep.
//! Set: HML #29 — Homelands | Scryfall ID: f81fca41-2315-4d12-b05c-d921a4c3c19e | Oracle ID: e6e1b51c-ba4b-4997-968a-751b4cba110f
// IMPLEMENTED — choose a basic land type on resolution until end of turn; draw once at the next turn's upkeep.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::JINX,
    oracle_id = "e6e1b51c-ba4b-4997-968a-751b4cba110f",
    scryfall_id = "f81fca41-2315-4d12-b05c-d921a4c3c19e",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Jinx",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::ChooseOne {
                choices: &[
                    Effect::ModifyTargets {
                        modifier: Modifier::SetLandType(subtypes::land::PLAINS),
                        duration: Duration::UntilEndOfTurn,
                    },
                    Effect::ModifyTargets {
                        modifier: Modifier::SetLandType(subtypes::land::ISLAND),
                        duration: Duration::UntilEndOfTurn,
                    },
                    Effect::ModifyTargets {
                        modifier: Modifier::SetLandType(subtypes::land::SWAMP),
                        duration: Duration::UntilEndOfTurn,
                    },
                    Effect::ModifyTargets {
                        modifier: Modifier::SetLandType(subtypes::land::MOUNTAIN),
                        duration: Duration::UntilEndOfTurn,
                    },
                    Effect::ModifyTargets {
                        modifier: Modifier::SetLandType(subtypes::land::FOREST),
                        duration: Duration::UntilEndOfTurn,
                    },
                ],
            },
            Effect::AtNextTurnsUpkeep {
                effects: &[Effect::draw(1)],
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::LAND)))
    )],
);
