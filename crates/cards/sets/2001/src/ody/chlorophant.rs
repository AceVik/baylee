//! Chlorophant — {G}{G}{G} — Creature — Elemental
//! Oracle: At the beginning of your upkeep, you may put a +1/+1 counter on this creature.
//! Oracle: Threshold — As long as there are seven or more cards in your graveyard, this creature has "At the beginning of your upkeep, you may put another +1/+1 counter on this creature."
//! Set: ODY #234 — Odyssey | Scryfall ID: c4fd0ea3-4134-4f7d-a0c8-01c49bacfcfc | Oracle ID: ad80d9a8-187f-4ecf-94be-e23ac3a57179
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CHLOROPHANT,
    oracle_id = "ad80d9a8-187f-4ecf-94be-e23ac3a57179",
    scryfall_id = "c4fd0ea3-4134-4f7d-a0c8-01c49bacfcfc",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Chlorophant",
        mana_cost = mana!("{G}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELEMENTAL],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
