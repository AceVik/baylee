//! Opportunist — {2}{R} — Creature — Human Soldier
//! Oracle: {T}: This creature deals 1 damage to target creature that was dealt damage this turn.
//! Set: TMP #194 — Tempest | Scryfall ID: ae0026c0-8f61-4485-8909-6a44c2ca9169 | Oracle ID: 5b5524f7-3c68-4c7b-be1b-745ffcad8006
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static DAMAGED_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::DealtDamageThisTurn]);

card!(
    index = index::OPPORTUNIST,
    oracle_id = "5b5524f7-3c68-4c7b-be1b-745ffcad8006",
    scryfall_id = "ae0026c0-8f61-4485-8909-6a44c2ca9169",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Opportunist",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SOLDIER],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[activated!(
        cost!(TapSelf),
        &[Effect::DealDamage {
            amount: Amount::Fixed(1),
            target: TargetSpec::Object(&DAMAGED_CREATURE),
        }],
        target = Some(TargetSpec::Object(&DAMAGED_CREATURE)),
    )],
);
