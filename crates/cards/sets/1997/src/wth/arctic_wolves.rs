//! Arctic Wolves — {3}{G}{G} — Creature — Wolf
//! Oracle: Cumulative upkeep {2} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Oracle: When this creature enters, draw a card.
//! Set: WTH #118 — Weatherlight | Scryfall ID: b5fb56a2-5138-4c31-aa4b-0824a1a24573 | Oracle ID: c9e7218f-07f2-46f3-bc63-eb5c0d32538b
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ARCTIC_WOLVES,
    oracle_id = "c9e7218f-07f2-46f3-bc63-eb5c0d32538b",
    scryfall_id = "b5fb56a2-5138-4c31-aa4b-0824a1a24573",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Arctic Wolves",
        mana_cost = mana!("{3}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WOLF],
        power = Some(4),
        toughness = Some(5),
    ),],
    abilities = &[
        triggered!(
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
        ),
        triggered!(Trigger::ETB, &[Effect::draw(1)]),
    ],
);
