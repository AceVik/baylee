//! Jasmine Dragon Tea Shop — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add one mana of any color. Spend this mana only to cast an Ally spell or activate an ability of an Ally source.
//! Oracle: {5}, {T}: Create a 1/1 white Ally creature token.
//! {T}: Add one mana of any color. Spend this mana only to cast an Ally spell or activate an ability of an Ally source.
//! {5}, {T}: Create a 1/1 white Ally creature token.
//! Set: TLA #270 — Avatar: The Last Airbender | Scryfall ID: da2c83d4-a95f-47ff-a08f-694eb78d6b9b | Oracle ID: d9a24444-289f-473f-9985-8df275257555
// IMPLEMENTED — the ally-restriction on the choice mana is enforced via
// restricted mana provenance (spendable only on Ally spells). The "or
// activate an ability of an Ally source" half of the restriction is a
// payment-solver refinement (spells only today).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

use crate::tokens::ALLY_1_1_WHITE as ALLY_TOKEN;

card!(
    index = index::JASMINE_DRAGON_TEA_SHOP,
    oracle_id = "d9a24444-289f-473f-9985-8df275257555",
    scryfall_id = "da2c83d4-a95f-47ff-a08f-694eb78d6b9b",
    faces = &[face!(
        name = "Jasmine Dragon Tea Shop",
        types = TypeSet::LAND,
    )],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(&[Effect::mana_choice(&[
            ManaColor::White,
            ManaColor::Blue,
            ManaColor::Black,
            ManaColor::Red,
            ManaColor::Green,
        ])
        .restricted(&Filter::HasSubtype(creature::ALLY), SpendRider::None,)]),
        activated!(
            cost!("{5}", TapSelf),
            &[Effect::CreateToken { token: &ALLY_TOKEN }]
        ),
    ],
);
