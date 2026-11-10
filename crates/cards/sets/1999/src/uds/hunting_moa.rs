//! Hunting Moa — {2}{G} — Creature — Bird Beast
//! Oracle: Echo {2}{G} (At the beginning of your upkeep, if this came under your control since the beginning of your last upkeep, sacrifice it unless you pay its echo cost.)
//! Oracle: When this creature enters or dies, put a +1/+1 counter on target creature.
//! Set: ARC #60 — Archenemy | Scryfall ID: d1cc7076-500c-4502-9344-266532c0de7f | Oracle ID: 102d73bc-94ed-4a57-9872-6f7b9bf282f9
// IMPLEMENTED — echo {2}{G}; entering the battlefield or dying each puts a +1/+1 counter on target creature.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HUNTING_MOA,
    oracle_id = "102d73bc-94ed-4a57-9872-6f7b9bf282f9",
    scryfall_id = "d1cc7076-500c-4502-9344-266532c0de7f",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Hunting Moa",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BIRD, subtypes::creature::BEAST],
        power = Some(3),
        toughness = Some(2),
    ),],
    abilities = &[
        AbilityDef::Echo {
            cost: mana!("{2}{G}"),
        },
        triggered!(
            Trigger::ETB,
            &[Effect::AddCounter {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(1),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        triggered!(
            Trigger::Dies(&Filter::This),
            &[Effect::AddCounter {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(1),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
    ],
);
