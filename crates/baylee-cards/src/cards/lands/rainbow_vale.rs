//! Rainbow Vale — (no cost) — Land
//! Oracle: {T}: Add one mana of any color. An opponent gains control of this land at the beginning of the next end step.
//! Set: ME1 #179 — Masters Edition | Scryfall ID: 51f8b918-ac13-4538-a39d-6553580bf39b | Oracle ID: 76695b15-d0ba-41eb-85f1-52ba5d14b8ba
// PARTIAL — {T}: Add one mana of any color. The control hand-off at the
// beginning of the next end step is NOT SUPPORTED and is dropped.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RAINBOW_VALE,
    oracle_id = "76695b15-d0ba-41eb-85f1-52ba5d14b8ba",
    scryfall_id = "51f8b918-ac13-4538-a39d-6553580bf39b",
    faces = &[face!(name = "Rainbow Vale", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "an opponent gains control of this land at the beginning of the next end step",
    ),
    abilities = &[
        // NOT SUPPORTED: "An opponent gains control of this land at the
        // beginning of the next end step." No variant registers a delayed
        // trigger at a future step, and Modifier::GainControl hands the
        // permanent to the effect's own controller, never to an opponent.
        mana_ability!(&[Effect::mana_of_any_color()]),
    ],
);
