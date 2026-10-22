//! Lichenthrope — {3}{G}{G} — Creature — Plant Fungus
//! Oracle: If damage would be dealt to this creature, put that many -1/-1 counters on it instead.
//! Oracle: At the beginning of your upkeep, remove a -1/-1 counter from this creature.
//! Set: VIS #112 — Visions | Scryfall ID: 76f0c356-a81d-41d4-a8b7-8c159146a8b8 | Oracle ID: ee7f698d-55c5-4a7a-803d-304febe6a758
// PARTIAL: removes a -1/-1 counter at the beginning of your upkeep; replacing damage dealt to Lichenthrope with that many -1/-1 counters is not yet supported.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// NOT SUPPORTED: If damage would be dealt to this creature, put that many -1/-1 counters on it instead.
card!(
    index = index::LICHENTHROPE,
    oracle_id = "ee7f698d-55c5-4a7a-803d-304febe6a758",
    scryfall_id = "76f0c356-a81d-41d4-a8b7-8c159146a8b8",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "removes a -1/-1 counter at the beginning of your upkeep; replacing damage dealt to Lichenthrope with that many -1/-1 counters is not yet supported"
    ),
    faces = &[face!(
        name = "Lichenthrope",
        mana_cost = mana!("{3}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::PLANT, subtypes::creature::FUNGUS],
        power = Some(5),
        toughness = Some(5),
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You,
        },
        &[Effect::RemoveCounterSelf {
            kind: CounterKind::M1M1,
            n: 1,
        }],
    )],
);
