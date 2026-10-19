//! Merfolk Assassin — {U}{U} — Creature — Merfolk Assassin
//! Oracle: {T}: Destroy target creature with islandwalk.
//! Set: TSB #25 — Time Spiral Timeshifted | Scryfall ID: 6a906b9b-5e7c-4300-98e4-5eec242c1306 | Oracle ID: 7edd536c-c043-4189-aa57-83d22eed3103

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const CREATURE_WITH_ISLANDWALK: Filter =
    Filter::And(&[Filter::CREATURE, Filter::HasKeyword(KeywordSet::ISLANDWALK)]);

card!(
    index = index::MERFOLK_ASSASSIN,
    oracle_id = "7edd536c-c043-4189-aa57-83d22eed3103",
    scryfall_id = "6a906b9b-5e7c-4300-98e4-5eec242c1306",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Merfolk Assassin",
        mana_cost = mana!("{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::MERFOLK, subtypes::creature::ASSASSIN],
        power = Some(1),
        toughness = Some(2),
        abilities = &[activated!(
            Cost::TAP,
            &[Effect::destroy(TargetSpec::Object(
                &CREATURE_WITH_ISLANDWALK,
            ))],
            target = Some(TargetSpec::Object(&CREATURE_WITH_ISLANDWALK)),
        )],
    ),],
);
