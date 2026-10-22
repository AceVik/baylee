//! Buried Alive — {2}{B} — Sorcery
//! Oracle: Search your library for up to three creature cards, put them into your graveyard, then shuffle.
//! Set: MH3 #273 — Modern Horizons 3 | Scryfall ID: 76878b19-2916-4f3f-bee5-20846bc9e2db | Oracle ID: 8203c621-a1a0-4865-8c9a-0d4064c86107
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BURIED_ALIVE,
    oracle_id = "8203c621-a1a0-4865-8c9a-0d4064c86107",
    scryfall_id = "76878b19-2916-4f3f-bee5-20846bc9e2db",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Buried Alive",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[Effect::SearchLibrary {
        filter: &Filter::CREATURE,
        finds: &[Find::GRAVEYARD, Find::GRAVEYARD, Find::GRAVEYARD],
        optional: true,
    }])],
);
