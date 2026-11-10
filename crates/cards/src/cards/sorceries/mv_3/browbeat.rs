//! Browbeat — {2}{R} — Sorcery
//! Oracle: Any player may have Browbeat deal 5 damage to them. If no one does, target player draws three cards.
//! Set: A25 #123 — Masters 25 | Scryfall ID: 7e8e9f60-f8ea-4f29-b7e2-ca87f8336fca | Oracle ID: 11cd0c42-4148-4d82-9172-b1a1967fe319
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BROWBEAT,
    oracle_id = "11cd0c42-4148-4d82-9172-b1a1967fe319",
    scryfall_id = "7e8e9f60-f8ea-4f29-b7e2-ca87f8336fca",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Browbeat",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
