//! Resistance Fighter — {W} — Creature — Human Soldier
//! Oracle: Sacrifice this creature: Prevent all combat damage target creature would deal this turn.
//! Set: 6ED #38 — Classic Sixth Edition | Scryfall ID: 1abf09d8-f972-4b81-80e9-70fb4a33ed56 | Oracle ID: a95a2d1c-70a9-4eb2-ae30-a07fab26a9c6
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RESISTANCE_FIGHTER,
    oracle_id = "a95a2d1c-70a9-4eb2-ae30-a07fab26a9c6",
    scryfall_id = "1abf09d8-f972-4b81-80e9-70fb4a33ed56",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Resistance Fighter",
        mana_cost = mana!("{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SOLDIER],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!(SacrificeSelf),
        &[Effect::continuous(
            &Filter::This,
            Modifier::PreventDamageFromIt,
            Duration::UntilEndOfTurn,
        )],
        target = Some(TargetSpec::Object(&Filter::CREATURE)),
    )],
);
