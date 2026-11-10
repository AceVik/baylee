//! Tempting Wurm — {1}{G} — Creature — Wurm
//! Oracle: When this creature enters, each opponent may put any number of artifact, creature, enchantment, and/or land cards from their hand onto the battlefield.
//! Set: ONS #291 — Onslaught | Scryfall ID: 857c2b6c-cfdf-4c88-a334-2937cb7db603 | Oracle ID: a2976426-108b-43a6-ab04-f71afac79893
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TEMPTING_WURM,
    oracle_id = "a2976426-108b-43a6-ab04-f71afac79893",
    scryfall_id = "857c2b6c-cfdf-4c88-a334-2937cb7db603",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Tempting Wurm",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WURM],
        power = Some(5),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
