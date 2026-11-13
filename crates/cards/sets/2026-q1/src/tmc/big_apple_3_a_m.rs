//! Big Apple, 3 a.m. — (no cost) — Land
//! Oracle: This land enters tapped. As it enters, choose a color.
//! Oracle: {T}: Add one mana of the chosen color.
//! Oracle: {5}, {T}: Create a 1/1 black Rat creature token for each opponent you have.
//! Set: TMC #42 — Teenage Mutant Ninja Turtles Eternal | Scryfall ID: b9cc16f9-fea3-4527-9b81-3c84ce9c5e01 | Oracle ID: dd01ef1f-f6be-498f-82e0-dc04833e685f

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BIG_APPLE_3_A_M,
    oracle_id = "dd01ef1f-f6be-498f-82e0-dc04833e685f",
    scryfall_id = "b9cc16f9-fea3-4527-9b81-3c84ce9c5e01",
    faces = &[face!(
        name = "Big Apple, 3 a.m.",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped, EnterModifier::ChooseColor],
    )],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana_chosen()]),
        activated!(
            cost!("{5}", TapSelf),
            &[Effect::CreateTokenN {
                token: &crate::generated_tokens::RAT_1_1_BLACK,
                amount: Amount::Players(PlayerRel::EachOpponent),
            }]
        ),
    ],
);
