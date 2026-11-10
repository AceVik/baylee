//! Rainbow Vale — (no cost) — Land
//! Oracle: {T}: Add one mana of any color. An opponent gains control of this land at the beginning of the next end step.
//! Set: ME1 #179 — Masters Edition | Scryfall ID: 51f8b918-ac13-4538-a39d-6553580bf39b | Oracle ID: 76695b15-d0ba-41eb-85f1-52ba5d14b8ba

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RAINBOW_VALE,
    oracle_id = "76695b15-d0ba-41eb-85f1-52ba5d14b8ba",
    scryfall_id = "51f8b918-ac13-4538-a39d-6553580bf39b",
    faces = &[face!(name = "Rainbow Vale", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[mana_ability!(&[
        Effect::mana_of_any_color(),
        Effect::AtNextEndStep {
            effects: &[Effect::ChangeController {
                new_controller: PlayerRel::Opponent
            }],
        }
    ]),],
);
