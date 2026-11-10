//! Catapult Squad — {1}{W} — Creature — Human Soldier
//! Oracle: Tap two untapped Soldiers you control: This creature deals 2 damage to target attacking or blocking creature.
//! Set: ONS #11 — Onslaught | Scryfall ID: 75a71d29-29eb-43c4-b0f3-457435e8f629 | Oracle ID: f1ff7a67-1add-4de1-bc25-62f63626c43a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CATAPULT_SQUAD,
    oracle_id = "f1ff7a67-1add-4de1-bc25-62f63626c43a",
    scryfall_id = "75a71d29-29eb-43c4-b0f3-457435e8f629",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Catapult Squad",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SOLDIER],
        power = Some(2),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
