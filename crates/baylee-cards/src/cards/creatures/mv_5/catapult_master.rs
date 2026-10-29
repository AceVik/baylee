//! Catapult Master — {3}{W}{W} — Creature — Human Soldier
//! Oracle: Tap five untapped Soldiers you control: Exile target creature.
//! Set: DDF #18 — Duel Decks: Elspeth vs. Tezzeret | Scryfall ID: 544df614-277e-43dd-8611-3ec1e1747f57 | Oracle ID: 02da2145-0edb-44be-8622-360182a8c25a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CATAPULT_MASTER,
    oracle_id = "02da2145-0edb-44be-8622-360182a8c25a",
    scryfall_id = "544df614-277e-43dd-8611-3ec1e1747f57",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Catapult Master",
        mana_cost = mana!("{3}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SOLDIER],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
