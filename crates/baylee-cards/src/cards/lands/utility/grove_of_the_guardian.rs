//! Grove of the Guardian — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {3}{G}{W}, {T}, Tap two untapped creatures you control, Sacrifice this land: Create an 8/8 green and white Elemental creature token with vigilance.
//! Set: RTR #240 — Return to Ravnica | Scryfall ID: 3cf60ca0-e01f-499c-8d04-d59050f38c33 | Oracle ID: f746612a-fbed-44ca-b2cc-5928e10cf4bb

use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;

card!(
    index = index::GROVE_OF_THE_GUARDIAN,
    oracle_id = "f746612a-fbed-44ca-b2cc-5928e10cf4bb",
    scryfall_id = "3cf60ca0-e01f-499c-8d04-d59050f38c33",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    faces = &[face!(name = "Grove of the Guardian", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // "Tap two untapped creatures" is two parts, one question each, the
        // way Time Sieve writes its five sacrifices.
        activated!(
            cost!(
                "{3}{G}{W}",
                TapSelf,
                TapOther(&Filter::YOUR_CREATURE),
                TapOther(&Filter::YOUR_CREATURE),
                SacrificeSelf
            ),
            &[Effect::CreateToken {
                token: &generated_tokens::ELEMENTAL_8_8_WHITE_GREEN_VIGILANCE
            }]
        ),
    ],
);
