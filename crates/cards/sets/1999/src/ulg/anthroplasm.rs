//! Anthroplasm — {2}{U}{U} — Creature — Shapeshifter
//! Oracle: This creature enters with two +1/+1 counters on it.
//! Oracle: {X}, {T}: Remove all +1/+1 counters from this creature and put X +1/+1 counters on it.
//! Set: ULG #25 — Urza's Legacy | Scryfall ID: 089e2bc7-0063-47bf-8f66-48bed6eb046b | Oracle ID: f7536d94-4062-4ccc-a086-1c47954b24c7
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ANTHROPLASM,
    oracle_id = "f7536d94-4062-4ccc-a086-1c47954b24c7",
    scryfall_id = "089e2bc7-0063-47bf-8f66-48bed6eb046b",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Anthroplasm",
        mana_cost = mana!("{2}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SHAPESHIFTER],
        power = Some(0),
        toughness = Some(0),
        enter_modifiers = &[EnterModifier::WithCounters {
            kind: CounterKind::P1P1,
            amount: Amount::Fixed(2),
        }],
    ),],
    abilities = &[activated!(
        cost!("{X}", TapSelf),
        &[
            Effect::RemoveAllCountersSelf {
                kind: CounterKind::P1P1,
            },
            Effect::AddCounter {
                kind: CounterKind::P1P1,
                amount: Amount::X,
            },
        ],
    )],
);
