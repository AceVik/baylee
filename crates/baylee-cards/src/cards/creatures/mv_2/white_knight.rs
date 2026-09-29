//! White Knight — {W}{W} — Creature — Human Knight
//! Oracle: First strike (This creature deals combat damage before creatures without first strike.)
//! Oracle: Protection from black (This creature can't be blocked, targeted, dealt damage, or enchanted by anything black.)
//! Set: DDG #9 — Duel Decks: Knights vs. Dragons | Scryfall ID: 660f69ef-c04f-4f53-80e6-8190549ab12a | Oracle ID: ddb021df-ae4a-4ac1-8353-d0b375761714
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WHITE_KNIGHT,
    oracle_id = "ddb021df-ae4a-4ac1-8353-d0b375761714",
    scryfall_id = "660f69ef-c04f-4f53-80e6-8190549ab12a",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "White Knight",
        mana_cost = mana!("{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::KNIGHT],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
