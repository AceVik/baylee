//! Recoil — {1}{U}{B} — Instant
//! Oracle: Return target permanent to its owner's hand. Then that player discards a card.
//! Set: DMR #197 — Dominaria Remastered | Scryfall ID: 9035dda8-5ad3-4169-b281-83d6ca55b71e | Oracle ID: 1579bf86-4400-4467-88e8-1c5ba1209804
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RECOIL,
    oracle_id = "1579bf86-4400-4467-88e8-1c5ba1209804",
    scryfall_id = "9035dda8-5ad3-4169-b281-83d6ca55b71e",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue]),
    faces = &[face!(
        name = "Recoil",
        mana_cost = mana!("{1}{U}{B}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
