//! Ominous Cemetery — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {5}, {T}, Exile this land: Target creature's owner shuffles it into their library.
//! Set: WHO #189 — Doctor Who | Scryfall ID: 2e843c57-fae3-4127-94e7-cad8c8bb9486 | Oracle ID: d002391f-1dad-4966-ac36-56cc3ec015b2

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::OMINOUS_CEMETERY,
    oracle_id = "d002391f-1dad-4966-ac36-56cc3ec015b2",
    scryfall_id = "2e843c57-fae3-4127-94e7-cad8c8bb9486",
    faces = &[face!(name = "Ominous Cemetery", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{5}", TapSelf, ExileSelf),
            &[Effect::ShuffleTargetsIntoOwnersLibraries],
            target = Some(TargetSpec::Object(&Filter::CREATURE))
        ),
    ],
);
