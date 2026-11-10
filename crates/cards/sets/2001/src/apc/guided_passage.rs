//! Guided Passage — {G}{U}{R} — Sorcery
//! Oracle: Reveal the cards in your library. An opponent chooses from among them a creature card, a land card, and a noncreature, nonland card. You put the chosen cards into your hand. Then shuffle.
//! Set: 2X2 #224 — Double Masters 2022 | Scryfall ID: b66c8ada-82a6-46d9-acd0-b4ff9e1e12bd | Oracle ID: f4b30f6c-09ac-407e-9c72-da64d212ce93
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GUIDED_PASSAGE,
    oracle_id = "f4b30f6c-09ac-407e-9c72-da64d212ce93",
    scryfall_id = "b66c8ada-82a6-46d9-acd0-b4ff9e1e12bd",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Red, Color::Blue]),
    faces = &[face!(
        name = "Guided Passage",
        mana_cost = mana!("{G}{U}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
