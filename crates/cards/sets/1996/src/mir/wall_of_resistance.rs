//! Wall of Resistance — {1}{W} — Creature — Wall
//! Oracle: Defender (This creature can't attack.)
//! Oracle: Flying
//! Oracle: At the beginning of each end step, if this creature was dealt damage this turn, put a +0/+1 counter on it.
//! Set: MIR #46 — Mirage | Scryfall ID: fd25787d-b90c-4b25-8259-1ac41d4dcd15 | Oracle ID: e6f5f80c-2620-4558-8a5f-8fc36cea4793
// IMPLEMENTED — defender, flying, and each-end-step trigger putting a +0/+1 counter on it if it was dealt damage this turn.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WALL_OF_RESISTANCE,
    oracle_id = "e6f5f80c-2620-4558-8a5f-8fc36cea4793",
    scryfall_id = "fd25787d-b90c-4b25-8259-1ac41d4dcd15",
    color_identity = ColorSet::from_slice(&[Color::White]),
    keywords = KeywordSet::DEFENDER.union(KeywordSet::FLYING),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Wall of Resistance",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WALL],
        power = Some(0),
        toughness = Some(3),
        keywords = KeywordSet::DEFENDER.union(KeywordSet::FLYING),
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::End,
            whose: PlayerRel::EachPlayer,
        },
        &[Effect::AddCounter {
            kind: CounterKind::Plus {
                power: 0,
                toughness: 1,
            },
            amount: Amount::Fixed(1),
        }],
        condition = Some(Condition::SourceMatches(&Filter::DealtDamageThisTurn)),
    )],
);
