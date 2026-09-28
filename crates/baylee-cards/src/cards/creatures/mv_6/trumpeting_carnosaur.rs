//! Trumpeting Carnosaur — {4}{R}{R} — Creature — Dinosaur
//! Oracle: Trample
//! Oracle: When this creature enters, discover 5.
//! Oracle: {2}{R}, Discard this card: It deals 3 damage to target creature or planeswalker.
//! Set: LCI #171 — The Lost Caverns of Ixalan | Scryfall ID: edc035ca-f0a3-4814-9405-d6dc6f048315 | Oracle ID: f2ef8bda-373d-4387-900d-0f1b6ccf72e9
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TRUMPETING_CARNOSAUR,
    oracle_id = "f2ef8bda-373d-4387-900d-0f1b6ccf72e9",
    scryfall_id = "edc035ca-f0a3-4814-9405-d6dc6f048315",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Trumpeting Carnosaur",
        mana_cost = mana!("{4}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DINOSAUR],
        power = Some(7),
        toughness = Some(6),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
