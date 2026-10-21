//! Sengir Bats — {1}{B}{B} — Creature — Bat
//! Oracle: Flying
//! Oracle: Whenever a creature dealt damage by this creature this turn dies, put a +1/+1 counter on this creature.
//! Set: HML #57a — Homelands | Scryfall ID: 3c745c2c-4311-412d-a137-02bf6d106e46 | Oracle ID: ade8f21a-a7bb-4bd9-a7f2-9fa3060ff1e8
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SENGIR_BATS,
    oracle_id = "ade8f21a-a7bb-4bd9-a7f2-9fa3060ff1e8",
    scryfall_id = "3c745c2c-4311-412d-a137-02bf6d106e46",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Sengir Bats",
        mana_cost = mana!("{1}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BAT],
        power = Some(1),
        toughness = Some(2),
    ),],
    abilities = &[triggered!(
        Trigger::DiesAfterDamageByThis(&Filter::CREATURE),
        &[Effect::AddCounter {
            kind: CounterKind::P1P1,
            amount: Amount::Fixed(1),
        }],
    )],
);
