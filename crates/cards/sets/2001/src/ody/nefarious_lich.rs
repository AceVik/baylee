//! Nefarious Lich — {B}{B}{B}{B} — Enchantment
//! Oracle: If damage would be dealt to you, exile that many cards from your graveyard instead. If you can't, you lose the game.
//! Oracle: If you would gain life, draw that many cards instead.
//! Oracle: When this enchantment leaves the battlefield, you lose the game.
//! Set: ODY #153 — Odyssey | Scryfall ID: d90f5a4e-8c3a-4803-bb59-d3d7c23761db | Oracle ID: d79e8ec4-b44b-4598-991c-7781dab55868
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::NEFARIOUS_LICH,
    oracle_id = "d79e8ec4-b44b-4598-991c-7781dab55868",
    scryfall_id = "d90f5a4e-8c3a-4803-bb59-d3d7c23761db",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Nefarious Lich",
        mana_cost = mana!("{B}{B}{B}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
