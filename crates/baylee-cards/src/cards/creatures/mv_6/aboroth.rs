//! Aboroth — {4}{G}{G} — Creature — Elemental
//! Oracle: Cumulative upkeep—Put a -1/-1 counter on this creature. (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Set: WTH #117 — Weatherlight | Scryfall ID: 8c72ac67-e4fb-49a1-b1e5-cd2e414bec28 | Oracle ID: 28f70f86-19a9-4811-bc10-423a05842d39
// NOT SUPPORTED: optionally declining the cumulative upkeep counter payment to sacrifice this creature instead is not yet modeled.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ABOROTH,
    oracle_id = "28f70f86-19a9-4811-bc10-423a05842d39",
    scryfall_id = "8c72ac67-e4fb-49a1-b1e5-cd2e414bec28",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "puts an age counter on itself at the beginning of your upkeep and puts a -1/-1 counter on itself for each age counter; optionally declining the cumulative upkeep payment to sacrifice this creature instead is not yet modeled",
    ),
    faces = &[face!(
        name = "Aboroth",
        mana_cost = mana!("{4}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELEMENTAL],
        power = Some(9),
        toughness = Some(9),
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You,
        },
        &[
            Effect::AddCounter {
                kind: counters::AGE,
                amount: Amount::Fixed(1),
            },
            Effect::AddCounter {
                kind: CounterKind::M1M1,
                amount: Amount::CountersOnSource(counters::AGE),
            },
        ],
        condition = Some(Condition::SourceMatches(&Filter::InZone(
            ZoneRef::Battlefield
        ))),
    )],
);
