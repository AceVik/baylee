//! Enlightened Tutor — {W} — Instant
//! Oracle: Search your library for an artifact or enchantment card, reveal it, then shuffle and put that card on top.
//! Set: DMR #6 — Dominaria Remastered | Scryfall ID: 1c9675fb-1a89-420f-aea8-50e0642f549c | Oracle ID: c5229c17-b7be-4b05-b683-f2277edc4849
// IMPLEMENTED — filtered tutor to the top of the library (reveal is M3).

static FIND: Filter = Filter::Or(&[Filter::ARTIFACT, Filter::ENCHANTMENT]);

use baylee_cards_dsl::prelude::*;

card! {
    index: 42,
    oracle_id: "c5229c17-b7be-4b05-b683-f2277edc4849",
    scryfall_id: "1c9675fb-1a89-420f-aea8-50e0642f549c",
    faces: &[face! {
        name: "Enlightened Tutor",
        mana_cost: baylee_core::mana!("{W}"),
        types: TypeSet::INSTANT,
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
    coverage: Coverage::Implemented,
    abilities: &[spell!(&[Effect::SearchLibrary {
            filter: &FIND,
            finds: &[Find::TOP_OF_LIBRARY],
            optional: false,
        }])],
}
