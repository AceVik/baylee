//! Sustaining Spirit — {1}{W} — Creature — Angel Spirit
//! Oracle: Cumulative upkeep {1}{W} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Oracle: Damage that would reduce your life total to less than 1 reduces it to 1 instead.
//! Set: ME2 #36 — Masters Edition II | Scryfall ID: 9fe9e2e6-c430-4806-b833-dc6d91f7ed80 | Oracle ID: 683c81ae-38cd-4db1-b7e7-a09d34b6431e
// IMPLEMENTED — cumulative upkeep {1}{W}; damage cannot reduce your life total below 1.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SUSTAINING_SPIRIT,
    oracle_id = "683c81ae-38cd-4db1-b7e7-a09d34b6431e",
    scryfall_id = "9fe9e2e6-c430-4806-b833-dc6d91f7ed80",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Sustaining Spirit",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ANGEL, subtypes::creature::SPIRIT],
        power = Some(0),
        toughness = Some(3),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[
                Effect::AddCounter {
                    kind: counters::AGE,
                    amount: Amount::Fixed(1),
                },
                Effect::PlayerMayPayManaEachThenElse {
                    player: PlayerRel::You,
                    each: mana!("{1}{W}"),
                    times: Amount::CountersOnSource(counters::AGE),
                    paid: &[],
                    unpaid: &[Effect::SacrificeSelf],
                },
            ],
            condition = Some(Condition::SourceMatches(&Filter::InZone(
                ZoneRef::Battlefield,
            ))),
        ),
        static_ability!(
            Filter::This,
            Modifier::DamageLifeLossFloor {
                players: PlayerRel::You,
                minimum: 1,
            }
        ),
    ],
);
