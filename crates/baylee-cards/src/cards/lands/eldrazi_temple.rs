//! Eldrazi Temple — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add {C}{C}. Spend this mana only to cast colorless Eldrazi spells or activate abilities of colorless Eldrazi.
//! Set: CMM #992 — Commander Masters | Scryfall ID: cbab7e1f-305e-4733-aa70-b27285740925 | Oracle ID: 7fab8d65-af51-47d3-8f10-2676bf6e8ba3
// PARTIAL — {C}, and {C}{C} restricted to colorless Eldrazi spells; the
// printed "or activate abilities of colorless Eldrazi" has no variant.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card!(
    index = index::ELDRAZI_TEMPLE,
    oracle_id = "7fab8d65-af51-47d3-8f10-2676bf6e8ba3",
    scryfall_id = "cbab7e1f-305e-4733-aa70-b27285740925",
    faces = &[face!(name = "Eldrazi Temple", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "ManaRestriction's filter names spells only, so \"or activate abilities of colorless Eldrazi\" is not expressible"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: "Spend this mana only to cast colorless Eldrazi
        // spells **or activate abilities of colorless Eldrazi**." — the
        // restriction is written for the spell half; nothing in the DSL
        // points a `ManaRestriction` at an ability.
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 2).restricted(
            &f!(colorless Filter::HasSubtype(creature::ELDRAZI)),
            SpendRider::None,
        )]),
    ],
);
