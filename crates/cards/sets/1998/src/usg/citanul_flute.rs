//! Citanul Flute — {5} — Artifact
//! Oracle: {X}, {T}: Search your library for a creature card with mana value X or less, reveal it, put it into your hand, then shuffle.
//! Set: 10E #315 — Tenth Edition | Scryfall ID: 5e3e0dff-d9f0-4222-a308-e86581c440cc | Oracle ID: 6b8183bb-20c4-4d45-b944-508a6a874bb3
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CITANUL_FLUTE,
    oracle_id = "6b8183bb-20c4-4d45-b944-508a6a874bb3",
    scryfall_id = "5e3e0dff-d9f0-4222-a308-e86581c440cc",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Citanul Flute",
        mana_cost = mana!("{5}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{X}", TapSelf),
        &[Effect::SearchLibraryOf {
            library: PlayerRel::You,
            owner_searches: true,
            filter: &Filter::CREATURE,
            mana_value: Some(ManaValueBound {
                cmp: ManaValueCmp::AtMost,
                amount: Amount::X,
            }),
            finds: &[Find::HAND],
            optional: false,
        }],
    )],
);
