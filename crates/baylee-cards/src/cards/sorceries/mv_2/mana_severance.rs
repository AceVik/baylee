//! Mana Severance — {1}{U} — Sorcery
//! Oracle: Search your library for any number of land cards, exile them, then shuffle.
//! Set: TMP #73 — Tempest | Scryfall ID: 854dc5e6-63f7-4c8b-83e5-a364f41c9a15 | Oracle ID: 34c66f52-dcbf-4d0a-bf7f-cb294bd93e9c
// PARTIAL: searches your library for any number of land cards, puts them into your graveyard, and shuffles; exiling the found land cards directly from the library search is not in SearchDest.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MANA_SEVERANCE,
    oracle_id = "34c66f52-dcbf-4d0a-bf7f-cb294bd93e9c",
    scryfall_id = "854dc5e6-63f7-4c8b-83e5-a364f41c9a15",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "searches your library for any number of land cards, puts them into your graveyard, and shuffles; exiling the found land cards directly from the library search is not in SearchDest",
    ),
    faces = &[face!(
        name = "Mana Severance",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[Effect::SearchLibrary {
        filter: &Filter::LAND,
        finds: &[Find::GRAVEYARD.any_number()],
        optional: true,
    }])],
);
