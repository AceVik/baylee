//! Springjack Pasture — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {4}, {T}: Create a 0/1 white Goat creature token.
//! Oracle: {T}, Sacrifice X Goats: Add X mana of any one color. You gain X life.
//! Set: C13 #326 — Commander 2013 | Scryfall ID: 035438b1-f794-41e5-9e2b-bc5136766cd5 | Oracle ID: 9eaadbbc-818b-4c21-9d4b-1bba48504d38
use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;

card!(
    index = index::SPRINGJACK_PASTURE,
    oracle_id = "9eaadbbc-818b-4c21-9d4b-1bba48504d38",
    scryfall_id = "035438b1-f794-41e5-9e2b-bc5136766cd5",
    faces = &[face!(name = "Springjack Pasture", types = TypeSet::LAND,)],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{4}", TapSelf),
            &[Effect::CreateToken {
                token: &generated_tokens::GOAT_0_1_WHITE,
            }],
        ),
        mana_ability!(
            cost!(
                TapSelf,
                SacrificeX(&Filter::HasSubtype(
                    baylee_core::generated::subtypes::creature::GOAT
                ))
            ),
            &[
                Effect::AddMana {
                    source: ManaSource::Choice(&[
                        ManaColor::White,
                        ManaColor::Blue,
                        ManaColor::Black,
                        ManaColor::Red,
                        ManaColor::Green
                    ]),
                    amount: Amount::X,
                    combination: false,
                    restriction: None
                },
                Effect::GainLife { amount: Amount::X },
            ],
        ),
    ],
);
