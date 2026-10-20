//! Diabolic Vision — {U}{B} — Sorcery
//! Oracle: Look at the top five cards of your library. Put one of them into your hand and the rest on top of your library in any order.
//! Set: DSC #87 — Duskmourn: House of Horror Commander | Scryfall ID: c360753a-af32-4f1d-a1c7-c3f3c075ecc0 | Oracle ID: 6b7b1a71-812d-4ff5-b38b-94d1cf634143
// PARTIAL: putting the remaining four cards on top of your library in any order not supported; puts rest on bottom.

use baylee_cards_dsl::prelude::*;

// NOT SUPPORTED: ... and the rest on top of your library in any order (LookAtTopPick puts the rest on the bottom).
card!(
    index = index::DIABOLIC_VISION,
    oracle_id = "6b7b1a71-812d-4ff5-b38b-94d1cf634143",
    scryfall_id = "c360753a-af32-4f1d-a1c7-c3f3c075ecc0",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue]),
    coverage = Coverage::Partial(
        "putting the remaining four cards on top of your library in any order not supported; puts rest on bottom"
    ),
    faces = &[face!(
        name = "Diabolic Vision",
        mana_cost = mana!("{U}{B}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[Effect::LookAtTopPick {
        count: Amount::Fixed(5),
        pick: 1,
        random: false,
    }])],
);
