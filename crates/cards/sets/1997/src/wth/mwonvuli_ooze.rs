//! Mwonvuli Ooze — {G} — Creature — Ooze
//! Oracle: Cumulative upkeep {2} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay {2} for each age counter on it.)
//! Oracle: Mwonvuli Ooze's power and toughness are each equal to 1 plus twice the number of age counters on it.
//! Set: WTH #135 — Weatherlight | Scryfall ID: aa9c6f65-93a1-4913-87e7-a17ebfcc7780 | Oracle ID: 42b2c6d0-e0da-41b0-8876-f17dbe5db79a
// PARTIAL — Has cumulative upkeep {2} as a 1/1; setting its power and toughness to 1 plus twice the number of age counters on it is not yet modeled.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MWONVULI_OOZE,
    oracle_id = "42b2c6d0-e0da-41b0-8876-f17dbe5db79a",
    scryfall_id = "aa9c6f65-93a1-4913-87e7-a17ebfcc7780",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "Has cumulative upkeep {2} as a 1/1; setting its power and toughness to 1 plus twice the number of age counters on it is not yet modeled."
    ),
    faces = &[face!(
        name = "Mwonvuli Ooze",
        mana_cost = mana!("{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::OOZE],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You
        },
        &[
            Effect::AddCounter {
                kind: counters::AGE,
                amount: Amount::Fixed(1)
            },
            Effect::PlayerMayPayOr {
                player: PlayerRel::You,
                mana: Amount::Times {
                    base: &Amount::CountersOnSource(counters::AGE),
                    factor: 2
                },
                effect: &Effect::SacrificeSelf
            }
        ],
        condition = Some(Condition::SourceMatches(&Filter::InZone(
            ZoneRef::Battlefield
        )))
    ),],
);
