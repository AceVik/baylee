//! Eumidian Hatchery — (no cost) — Land
//! Oracle: {T}, Pay 1 life: Add {B}. Put a hatchling counter on this land.
//! Oracle: When this land is put into a graveyard from the battlefield, for each hatchling counter on it, create a 1/1 black Insect creature token with flying.
//! Set: EOC #20 — Edge of Eternities Commander | Scryfall ID: 25b57aaf-04fa-463d-8516-40fccd24d6ed | Oracle ID: 5b6d933e-2830-4f5a-b244-a421aa9615dc

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;

card!(
    index = index::EUMIDIAN_HATCHERY,
    oracle_id = "5b6d933e-2830-4f5a-b244-a421aa9615dc",
    scryfall_id = "25b57aaf-04fa-463d-8516-40fccd24d6ed",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(name = "Eumidian Hatchery", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(
            cost!(TapSelf, PayLife(1)),
            &[
                Effect::mana(ManaColor::Black, 1),
                Effect::AddCounter {
                    kind: counters::HATCHLING,
                    amount: Amount::Fixed(1),
                },
            ],
        ),
        triggered!(
            Trigger::Dies(&Filter::This),
            &[Effect::CreateTokenN {
                token: &generated_tokens::INSECT_1_1_BLACK_FLYING,
                amount: Amount::CountersOnSource(counters::HATCHLING)
            }]
        )
    ],
);
