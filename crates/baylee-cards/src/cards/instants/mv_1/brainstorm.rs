//! Brainstorm — {U} — Instant
//! Oracle: Draw three cards, then put two cards from your hand on top of your library in any order.
//! Set: TLE #155 — Avatar: The Last Airbender Eternal | Scryfall ID: b5545882-6963-4729-b2c6-fb4bdc75ffcc | Oracle ID: 36cd2364-d113-47d1-b2c4-b088d9eb88dd
// IMPLEMENTED — draw 3, put 2 back on top (chosen order).

use baylee_cards_dsl::prelude::*;

card! {
    index: 15,
    oracle_id: "36cd2364-d113-47d1-b2c4-b088d9eb88dd",
    scryfall_id: "b5545882-6963-4729-b2c6-fb4bdc75ffcc",
    faces: &[face! {
        name: "Brainstorm",
        mana_cost: baylee_core::mana!("{U}"),
        types: TypeSet::INSTANT,
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    coverage: Coverage::Implemented,
    abilities: &[spell!(&[
            Effect::DrawCards {
                amount: Amount::Fixed(3),
            },
            Effect::PutFromHandOnTop { count: 2 },
        ])],
}

// Engine-level coverage via s4 scenario tests: draw 3 then put 2 back;
// the top card of the library afterwards is the second chosen card.
