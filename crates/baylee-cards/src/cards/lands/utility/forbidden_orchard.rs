//! Forbidden Orchard — (no cost) — Land
//! Oracle: {T}: Add one mana of any color.
//! Oracle: Whenever you tap this land for mana, target opponent creates a 1/1 colorless Spirit creature token.
//! Set: 2X2 #323 — Double Masters 2022 | Scryfall ID: 17db644c-1acf-477d-9c20-f72221f1108a | Oracle ID: cfd60d1f-9832-4408-b84e-0fd3018b015b

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;

card!(
    index = index::FORBIDDEN_ORCHARD,
    oracle_id = "cfd60d1f-9832-4408-b84e-0fd3018b015b",
    scryfall_id = "17db644c-1acf-477d-9c20-f72221f1108a",
    faces = &[face!(name = "Forbidden Orchard", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana_of_any_color()]),
        triggered!(
            Trigger::TappedForMana {
                by: PlayerRel::You,
                filter: &Filter::This
            },
            &[Effect::CreateTokensFor {
                player: PlayerRel::Chosen,
                token: &generated_tokens::SPIRIT_1_1,
                amount: Amount::Fixed(1)
            }],
            targets = Some(TargetReq::one(TargetSpec::AnyOpponent))
        ),
    ],
);
