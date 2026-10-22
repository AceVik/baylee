//! Volunteer Reserves — {1}{W} — Creature — Human Soldier
//! Oracle: Banding (Any creatures with banding, and up to one without, can attack in a band. Bands are blocked as a group. If any creatures with banding you control are blocking or being blocked by a creature, you divide that creature's combat damage, not its controller, among any of the creatures it's being blocked by or is blocking.)
//! Oracle: Cumulative upkeep {1} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Set: WTH #29 — Weatherlight | Scryfall ID: 5344911f-25e8-45ce-87b9-607e42db0139 | Oracle ID: a2dabd52-0b75-42d7-a07d-0df7b38646e2
// IMPLEMENTED — Banding and cumulative upkeep {1}.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::VOLUNTEER_RESERVES,
    oracle_id = "a2dabd52-0b75-42d7-a07d-0df7b38646e2",
    scryfall_id = "5344911f-25e8-45ce-87b9-607e42db0139",
    color_identity = ColorSet::from_slice(&[Color::White]),
    keywords = KeywordSet::BANDING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Volunteer Reserves",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SOLDIER],
        power = Some(2),
        toughness = Some(4),
        keywords = KeywordSet::BANDING,
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
                mana: Amount::CountersOnSource(counters::AGE),
                effect: &Effect::SacrificeSelf
            }
        ],
        condition = Some(Condition::SourceMatches(&Filter::InZone(
            ZoneRef::Battlefield
        )))
    ),],
);
