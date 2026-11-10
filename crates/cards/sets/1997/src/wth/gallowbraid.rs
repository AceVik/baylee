//! Gallowbraid — {3}{B}{B} — Legendary Creature — Phyrexian Horror
//! Oracle: Trample
//! Oracle: Cumulative upkeep—Pay 1 life. (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Set: WTH #70 — Weatherlight | Scryfall ID: 8df86192-6374-42ac-94bc-95e2e8284bd6 | Oracle ID: 4a4418fa-7574-4027-aa09-4ca789be15ba
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GALLOWBRAID,
    oracle_id = "4a4418fa-7574-4027-aa09-4ca789be15ba",
    scryfall_id = "8df86192-6374-42ac-94bc-95e2e8284bd6",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    commander = CommanderRule::Legendary,
    keywords = KeywordSet::TRAMPLE,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Gallowbraid",
        mana_cost = mana!("{3}{B}{B}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::PHYREXIAN, subtypes::creature::HORROR],
        power = Some(5),
        toughness = Some(5),
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
            Effect::PlayerMayPayLifeOr {
                player: PlayerRel::You,
                life: Amount::CountersOnSource(counters::AGE),
                effect: &Effect::SacrificeSelf
            }
        ],
        condition = Some(Condition::SourceMatches(&Filter::InZone(
            ZoneRef::Battlefield
        )))
    ),],
);
