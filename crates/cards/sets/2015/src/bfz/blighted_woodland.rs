//! Blighted Woodland — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {3}{G}, {T}, Sacrifice this land: Search your library for up to two basic land cards, put them onto the battlefield tapped, then shuffle.
//! Set: CLB #881 — Commander Legends: Battle for Baldur's Gate | Scryfall ID: 94e0ad38-31b3-46ba-9999-9b6ff57906ae | Oracle ID: 02679a2e-303d-412f-87d8-0a37a8ca259c
// IMPLEMENTED — the {C} mana ability plus the {3}{G} land fetch ("up to
// two basic land cards onto the battlefield tapped") as one SearchLibrary
// with two BATTLEFIELD_TAPPED finds and `optional` for the "up to".

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BLIGHTED_WOODLAND,
    oracle_id = "02679a2e-303d-412f-87d8-0a37a8ca259c",
    scryfall_id = "94e0ad38-31b3-46ba-9999-9b6ff57906ae",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(name = "Blighted Woodland", types = TypeSet::LAND,),],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{3}{G}", TapSelf, SacrificeSelf),
            &[Effect::SearchLibrary {
                filter: &Filter::BASIC_LAND,
                finds: &[Find::BATTLEFIELD_TAPPED, Find::BATTLEFIELD_TAPPED],
                optional: true,
            }]
        ),
    ],
);
