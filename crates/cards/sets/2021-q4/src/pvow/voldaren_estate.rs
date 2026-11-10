//! Voldaren Estate — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}, Pay 1 life: Add one mana of any color. Spend this mana only to cast a Vampire spell.
//! Oracle: {5}, {T}: Create a Blood token. This ability costs {1} less to activate for each Vampire you control. (It's an artifact with "{1}, {T}, Discard a card, Sacrifice this token: Draw a card.")
//! Set: LCC #369 — The Lost Caverns of Ixalan Commander | Scryfall ID: 1fcbc704-4fb3-49c3-9040-aa1681e86e2c | Oracle ID: fb0c0426-f1a6-4e52-9242-627786d3119a

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card!(
    index = index::VOLDAREN_ESTATE,
    oracle_id = "fb0c0426-f1a6-4e52-9242-627786d3119a",
    scryfall_id = "1fcbc704-4fb3-49c3-9040-aa1681e86e2c",
    faces = &[face!(name = "Voldaren Estate", types = TypeSet::LAND,)],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(
            cost!(TapSelf, PayLife(1)),
            &[Effect::mana_of_any_color()
                .restricted(&Filter::HasSubtype(creature::VAMPIRE), SpendRider::None)]
        ),
        activated!(
            cost!("{5}", TapSelf),
            &[Effect::CreateToken {
                token: &crate::tokens::BLOOD
            }],
            cost_reduction = Some(CostReduction::PerCount {
                amount: Amount::CountOf {
                    filter: &Filter::And(&[
                        Filter::HasSubtype(creature::VAMPIRE),
                        Filter::ControlledByYou
                    ]),
                    zone: ZoneSel::Battlefield
                },
                each: 1
            }),
        ),
    ],
);
