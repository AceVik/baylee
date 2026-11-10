//! Rashida Scalebane — {3}{W}{W} — Legendary Creature — Human Soldier
//! Oracle: {T}: Destroy target attacking or blocking Dragon. It can't be regenerated. You gain life equal to its power.
//! Set: MIR #35 — Mirage | Scryfall ID: ebb48053-da60-477a-b1eb-a9ab9ea682af | Oracle ID: 4aff0f09-0607-41b7-8f60-4ef35ff6183e
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const ATTACKING_OR_BLOCKING_DRAGON: Filter = Filter::And(&[
    Filter::HasSubtype(subtypes::creature::DRAGON),
    Filter::Or(&[Filter::Attacking, Filter::Blocking]),
]);

card!(
    index = index::RASHIDA_SCALEBANE,
    oracle_id = "4aff0f09-0607-41b7-8f60-4ef35ff6183e",
    scryfall_id = "ebb48053-da60-477a-b1eb-a9ab9ea682af",
    color_identity = ColorSet::from_slice(&[Color::White]),
    commander = CommanderRule::Legendary,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Rashida Scalebane",
        mana_cost = mana!("{3}{W}{W}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SOLDIER],
        power = Some(3),
        toughness = Some(4),
    ),],
    abilities = &[activated!(
        Cost::TAP,
        &[
            Effect::Destroy {
                target: TargetSpec::Object(&ATTACKING_OR_BLOCKING_DRAGON),
                no_regen: true,
            },
            Effect::GainLife {
                amount: Amount::TargetPower,
            },
        ],
        target = Some(TargetSpec::Object(&ATTACKING_OR_BLOCKING_DRAGON)),
    )],
);
