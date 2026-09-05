//! Rampant Growth — {1}{G} — Sorcery
//! Oracle: Search your library for a basic land card, put that card onto the battlefield tapped, then shuffle.
//! Set: TDC #265 — Tarkir: Dragonstorm Commander | Scryfall ID: b7c47024-5e08-4b17-b41a-7647f8b814b9 | Oracle ID: 8539f295-5d58-4436-a73a-b9277c4c7795
// IMPLEMENTED — one basic land, tapped. Deliberately *not* Farseek's filter:
// "basic land card" is the supertype, so Blood Crypt is legal for Farseek and
// illegal here.

use baylee_cards_dsl::prelude::*;

card! {
    index: 1356,
    oracle_id: "8539f295-5d58-4436-a73a-b9277c4c7795",
    scryfall_id: "b7c47024-5e08-4b17-b41a-7647f8b814b9",
    color_identity: ColorSet::from_slice(&[Color::Green]),
    coverage: Coverage::Implemented,
    faces: &[
    face! {
        name: "Rampant Growth",
        mana_cost: baylee_core::mana!("{1}{G}"),
        types: TypeSet::SORCERY,
    },
    ],
    abilities: &[spell!(&[Effect::SearchLibrary {
        filter: &Filter::BASIC_LAND,
        finds: &[Find::BATTLEFIELD_TAPPED],
        optional: false,
    }])],
}
