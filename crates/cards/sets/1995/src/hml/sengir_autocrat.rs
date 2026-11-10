//! Sengir Autocrat — {3}{B} — Creature — Human
//! Oracle: When this creature enters, create three 0/1 black Serf creature tokens.
//! Oracle: When this creature leaves the battlefield, exile all Serf tokens.
//! Set: EMA #105 — Eternal Masters | Scryfall ID: 83fa8a17-ff86-465e-9092-0968081f4e28 | Oracle ID: 0561043a-3896-4bb1-8ff5-ee674ddfea2a
// IMPLEMENTED

use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const SERF_TOKEN: Filter = Filter::And(&[
    Filter::IsToken,
    Filter::HasSubtype(subtypes::creature::SERF),
]);

card!(
    index = index::SENGIR_AUTOCRAT,
    oracle_id = "0561043a-3896-4bb1-8ff5-ee674ddfea2a",
    scryfall_id = "83fa8a17-ff86-465e-9092-0968081f4e28",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Sengir Autocrat",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[
        triggered!(
            Trigger::ETB,
            &[Effect::CreateTokenN {
                token: &generated_tokens::SERF_0_1_BLACK,
                amount: Amount::Fixed(3),
            }]
        ),
        triggered!(
            Trigger::LeavesBattlefield(&Filter::This),
            &[Effect::ExileAll {
                filter: &SERF_TOKEN,
                linked: false,
            }]
        ),
    ],
);
