//! Library of Alexandria — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Draw a card. Activate only if you have exactly seven cards in hand.
//! Set: VMA #303 — Vintage Masters | Scryfall ID: e5145f31-a4ac-44ef-8f85-e4d95f2c9ff5 | Oracle ID: 2111588d-9af5-4a33-989e-b074d83f0463
// IMPLEMENTED — both halves, the draw behind the exact hand size it prints.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LIBRARY_OF_ALEXANDRIA,
    oracle_id = "2111588d-9af5-4a33-989e-b074d83f0463",
    scryfall_id = "e5145f31-a4ac-44ef-8f85-e4d95f2c9ff5",
    faces = &[face!(name = "Library of Alexandria", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // **Exactly** seven, which is the whole card: an "at most" reading
        // would hand the draw to every hand below seven and an "at least"
        // reading would never stop, and the printed number is the one that
        // makes drawing put you out of range of your own land.
        activated!(
            Cost::TAP,
            &[Effect::draw(1)],
            condition = Some(Condition::HandSizeExactly(7)),
        ),
    ],
);
