//! Renegade Warlord — {4}{R} — Creature — Human Soldier
//! Oracle: First strike
//! Oracle: Whenever this creature attacks, each other attacking creature gets +1/+0 until end of turn.
//! Set: TPR #151 — Tempest Remastered | Scryfall ID: 7f9db6c9-b84d-43de-88b1-fe60ba8dcfd4 | Oracle ID: ae63849d-7352-4a2b-9d64-f3005146bd22
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static OTHER_ATTACKING_CREATURE: Filter =
    Filter::And(&[Filter::CREATURE, Filter::Attacking, Filter::Another]);

card!(
    index = index::RENEGADE_WARLORD,
    oracle_id = "ae63849d-7352-4a2b-9d64-f3005146bd22",
    scryfall_id = "7f9db6c9-b84d-43de-88b1-fe60ba8dcfd4",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Renegade Warlord",
        mana_cost = mana!("{4}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SOLDIER],
        power = Some(3),
        toughness = Some(3),
        keywords = KeywordSet::FIRST_STRIKE,
    ),],
    abilities = &[triggered!(
        Trigger::Attacks(&Filter::This),
        &[Effect::PumpFilter {
            filter: &OTHER_ATTACKING_CREATURE,
            controlled_by: None,
            power: Amount::Fixed(1),
            toughness: Amount::Fixed(0),
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        }],
    )],
);
