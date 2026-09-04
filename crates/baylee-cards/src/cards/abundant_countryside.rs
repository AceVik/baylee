//! Abundant Countryside — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add one mana of any color. Spend this mana only to cast a creature spell.
//! Oracle: {6}, {T}: Create a 1/1 colorless Shapeshifter creature token with changeling. (It's every creature type.)
//! Set: ECC #22 — Lorwyn Eclipsed Commander | Scryfall ID: 37478625-dd07-476d-bd9b-b2e0d71ac0d1 | Oracle ID: e3eb6f90-ccfc-41e7-bff6-0b378226bc7e
// IMPLEMENTED — {C} mana, creature-restricted any-color mana, and {6},{T} changeling token creation.

use baylee_cards_dsl::prelude::*;

use crate::tokens::SHAPESHIFTER_1_1_CHANGELING as SHAPESHIFTER_TOKEN;

card! {
    index: 201,
    oracle_id: "e3eb6f90-ccfc-41e7-bff6-0b378226bc7e",
    scryfall_id: "37478625-dd07-476d-bd9b-b2e0d71ac0d1",
    coverage: Coverage::Implemented,
    faces: &[
    face! {
        name: "Abundant Countryside",
        types: TypeSet::LAND,
    },
    ],
    abilities: &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(&[Effect::mana_of_any_color().restricted(&Filter::CREATURE, SpendRider::None)]),
        activated!(
            Cost {
                mana: baylee_core::mana!("{6}"),
                parts: &[CostPart::TapSelf],
            },
            &[Effect::CreateToken {
                token: &SHAPESHIFTER_TOKEN,
            }],
        ),
    ],
}

// Engine-level coverage lives in baylee-engine: restricted mana pool
// enforcement on creature spells and token creation.
