//! Bog Down — {2}{B} — Sorcery
//! Oracle: Kicker—Sacrifice two lands. (You may sacrifice two lands in addition to any other costs as you cast this spell.)
//! Oracle: Target player discards two cards. If this spell was kicked, that player discards three cards instead.
//! Set: PLS #39 — Planeshift | Scryfall ID: 8752a605-38f8-4d75-b122-063a788dff6e | Oracle ID: 8574f57a-768d-42a0-ac8c-cc0a1de181e0
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BOG_DOWN,
    oracle_id = "8574f57a-768d-42a0-ac8c-cc0a1de181e0",
    scryfall_id = "8752a605-38f8-4d75-b122-063a788dff6e",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Bog Down",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
