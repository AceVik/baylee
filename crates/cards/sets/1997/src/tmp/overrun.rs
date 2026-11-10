//! Overrun — {2}{G}{G}{G} — Sorcery
//! Oracle: Creatures you control get +3/+3 and gain trample until end of turn. (Each of those creatures can deal excess combat damage to the player or planeswalker it's attacking.)
//! Set: FDN #230 — Foundations | Scryfall ID: 1d8e9cbb-8bf4-4a48-a58e-79deb3abdf7f | Oracle ID: 204f9afe-c20b-4933-b5cd-aa572784762a
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::OVERRUN,
    oracle_id = "204f9afe-c20b-4933-b5cd-aa572784762a",
    scryfall_id = "1d8e9cbb-8bf4-4a48-a58e-79deb3abdf7f",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Overrun",
        mana_cost = mana!("{2}{G}{G}{G}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[Effect::PumpFilter {
        filter: &Filter::YOUR_CREATURE,
        controlled_by: None,
        power: Amount::Fixed(3),
        toughness: Amount::Fixed(3),
        keywords: KeywordSet::TRAMPLE,
        duration: Duration::UntilEndOfTurn,
    }])],
);
