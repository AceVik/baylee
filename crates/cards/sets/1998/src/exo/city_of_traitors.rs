//! City of Traitors — (no cost) — Land
//! Oracle: When you play another land, sacrifice this land.
//! Oracle: {T}: Add {C}{C}.
//! Set: TPR #237 — Tempest Remastered | Scryfall ID: 71624139-a255-48be-93ca-594a4beba487 | Oracle ID: f161111d-9747-47b3-bb10-3c8bded32e21

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CITY_OF_TRAITORS,
    oracle_id = "f161111d-9747-47b3-bb10-3c8bded32e21",
    scryfall_id = "71624139-a255-48be-93ca-594a4beba487",
    faces = &[face!(name = "City of Traitors", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        triggered!(Trigger::PlaysAnotherLand, &[Effect::SacrificeSelf]),
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 2)]),
    ],
);
