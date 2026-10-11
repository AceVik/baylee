//! Bucolic Ranch — (no cost) — Land — Desert
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add one mana of any color. Spend this mana only to cast a Mount spell.
//! Oracle: {3}, {T}: Look at the top card of your library. If it's a Mount card, you may reveal it and put it into your hand. If you don't put it into your hand, you may put it on the bottom of your library.
//! Set: OTJ #265 — Outlaws of Thunder Junction | Scryfall ID: 6c4f6b81-53d0-49fb-b404-c2ad67de7493 | Oracle ID: 8f5902bf-4bc4-4d0c-84ea-a425307a4eb2

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BUCOLIC_RANCH,
    oracle_id = "8f5902bf-4bc4-4d0c-84ea-a425307a4eb2",
    scryfall_id = "6c4f6b81-53d0-49fb-b404-c2ad67de7493",
    faces = &[face!(
        name = "Bucolic Ranch",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::DESERT],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(&[Effect::mana_of_any_color().restricted(
            &Filter::HasSubtype(subtypes::creature::MOUNT),
            SpendRider::None
        )]),
        activated!(
            cost!("{3}", TapSelf),
            &[Effect::LookAtTopMayTake {
                filter: &Filter::HasSubtype(subtypes::creature::MOUNT),
                rest: LookRest::MayBottom,
                then: &[],
            }]
        ),
    ],
);
