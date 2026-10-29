//! Centaur Chieftain — {3}{G} — Creature — Centaur
//! Oracle: Haste
//! Oracle: Threshold — As long as there are seven or more cards in your graveyard, this creature has "When this creature enters, creatures you control get +1/+1 and gain trample until end of turn."
//! Set: EMA #160 — Eternal Masters | Scryfall ID: cb106dd9-32f4-4716-9578-8e7164cc704a | Oracle ID: 3010ff0c-8ee6-4df3-96c1-a58096586469
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CENTAUR_CHIEFTAIN,
    oracle_id = "3010ff0c-8ee6-4df3-96c1-a58096586469",
    scryfall_id = "cb106dd9-32f4-4716-9578-8e7164cc704a",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Centaur Chieftain",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::CENTAUR],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
