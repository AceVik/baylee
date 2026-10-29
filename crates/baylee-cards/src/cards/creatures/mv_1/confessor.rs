//! Confessor — {W} — Creature — Human Cleric
//! Oracle: Whenever a player discards a card, you may gain 1 life.
//! Set: ODY #15 — Odyssey | Scryfall ID: 984c0bdb-e9e2-435e-aa68-2dbad52c3dbd | Oracle ID: b78973fa-a4cb-4927-8a75-6b1c963c1c0c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CONFESSOR,
    oracle_id = "b78973fa-a4cb-4927-8a75-6b1c963c1c0c",
    scryfall_id = "984c0bdb-e9e2-435e-aa68-2dbad52c3dbd",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Confessor",
        mana_cost = mana!("{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
