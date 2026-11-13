//! Restock — {3}{G}{G} — Sorcery
//! Oracle: Return two target cards from your graveyard to your hand. Exile Restock.
//! Set: M15 #195 — Magic 2015 | Scryfall ID: 343994a6-b4f1-4ec5-997a-307b26601434 | Oracle ID: 8b641cd3-bb08-4f2e-9dad-24f58a9c32e1
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RESTOCK,
    oracle_id = "8b641cd3-bb08-4f2e-9dad-24f58a9c32e1",
    scryfall_id = "343994a6-b4f1-4ec5-997a-307b26601434",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Restock",
        mana_cost = mana!("{3}{G}{G}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
