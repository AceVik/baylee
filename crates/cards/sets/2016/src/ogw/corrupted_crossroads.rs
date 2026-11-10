//! Corrupted Crossroads — (no cost) — Land
//! Oracle: {T}: Add {C}. ({C} represents colorless mana.)
//! Oracle: {T}, Pay 1 life: Add one mana of any color. Spend this mana only to cast a spell with devoid.
//! Set: OGW #169 — Oath of the Gatewatch | Scryfall ID: 7043e193-8051-4f1b-9633-9c1f250d3607 | Oracle ID: 6276a985-7630-476d-a94f-6c6adc88f6c4

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CORRUPTED_CROSSROADS,
    oracle_id = "6276a985-7630-476d-a94f-6c6adc88f6c4",
    scryfall_id = "7043e193-8051-4f1b-9633-9c1f250d3607",
    faces = &[face!(name = "Corrupted Crossroads", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(
            cost!(TapSelf, PayLife(1)),
            &[Effect::mana_of_any_color()
                .restricted(&Filter::HasKeyword(KeywordSet::DEVOID), SpendRider::None)]
        ),
    ],
);
