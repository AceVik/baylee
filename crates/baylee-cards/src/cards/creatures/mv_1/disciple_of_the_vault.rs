//! Disciple of the Vault — {B} — Creature — Human Cleric
//! Oracle: Whenever an artifact is put into a graveyard from the battlefield, you may have target opponent lose 1 life.
//! Set: 2XM #86 — Double Masters | Scryfall ID: 4c539843-4e3f-47a7-92e1-412eaaa2d9c5 | Oracle ID: c8625113-0ce4-4454-83a1-25c31b8bfb9a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DISCIPLE_OF_THE_VAULT,
    oracle_id = "c8625113-0ce4-4454-83a1-25c31b8bfb9a",
    scryfall_id = "4c539843-4e3f-47a7-92e1-412eaaa2d9c5",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Disciple of the Vault",
        mana_cost = mana!("{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
