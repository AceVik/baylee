//! Hanweir Garrison — {2}{R} — Creature — Human Soldier
//! Oracle: Whenever this creature attacks, create two 1/1 red Human creature tokens that are tapped and attacking.
//! Oracle: (Melds with Hanweir Battlements.)
//! Set: INR #157 — Innistrad Remastered | Scryfall ID: 1811b4fa-fed6-46ea-a6de-bb7624a5b1de | Oracle ID: 7cb29569-48e1-4782-9906-fad155ebfafe
// IMPLEMENTED — attack trigger creates two tapped and attacking Humans.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HANWEIR_GARRISON,
    oracle_id = "7cb29569-48e1-4782-9906-fad155ebfafe",
    scryfall_id = "1811b4fa-fed6-46ea-a6de-bb7624a5b1de",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    abilities = &[triggered!(
        Trigger::Attacks(&Filter::This),
        &[Effect::CreateTokenAttacking {
            token: &crate::generated_tokens::HUMAN_1_1_RED,
            amount: Amount::Fixed(2),
            sacrifice_at_next_end_step: false,
        }]
    )],
    faces = &[face!(
        name = "Hanweir Garrison",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SOLDIER],
        power = Some(2),
        toughness = Some(3),
    ),],
);
