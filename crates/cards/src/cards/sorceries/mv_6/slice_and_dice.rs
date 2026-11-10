//! Slice and Dice — {4}{R}{R} — Sorcery
//! Oracle: Slice and Dice deals 4 damage to each creature.
//! Oracle: Cycling {2}{R} ({2}{R}, Discard this card: Draw a card.)
//! Oracle: When you cycle this card, you may have it deal 1 damage to each creature.
//! Set: CMM #255 — Commander Masters | Scryfall ID: dad077c9-abc7-4296-a5d1-9441aa6c38c0 | Oracle ID: 463fc961-d34e-4f40-b383-5b78a0fcb5c8
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SLICE_AND_DICE,
    oracle_id = "463fc961-d34e-4f40-b383-5b78a0fcb5c8",
    scryfall_id = "dad077c9-abc7-4296-a5d1-9441aa6c38c0",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Slice and Dice",
        mana_cost = mana!("{4}{R}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
