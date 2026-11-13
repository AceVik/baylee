//! Transcendence — {3}{W}{W}{W} — Enchantment
//! Oracle: You don't lose the game for having 0 or less life.
//! Oracle: When you have 20 or more life, you lose the game.
//! Oracle: Whenever you lose life, you gain 2 life for each 1 life you lost. (Damage dealt to you causes you to lose life.)
//! Set: TOR #20 — Torment | Scryfall ID: 84e8bee2-9e3d-493c-a937-645bf3dcf0db | Oracle ID: f9279f8f-3d19-4f5b-b194-cb8c65313b7e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TRANSCENDENCE,
    oracle_id = "f9279f8f-3d19-4f5b-b194-cb8c65313b7e",
    scryfall_id = "84e8bee2-9e3d-493c-a937-645bf3dcf0db",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Transcendence",
        mana_cost = mana!("{3}{W}{W}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
