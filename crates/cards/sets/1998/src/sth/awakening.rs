//! Awakening — {2}{G}{G} — Enchantment
//! Oracle: At the beginning of each upkeep, untap all creatures and lands.
//! Set: STH #101 — Stronghold | Scryfall ID: 59612c02-6028-4953-ac4d-aca94b0ce4b9 | Oracle ID: 5563427d-ffdc-4bea-ad7d-1605894b8a6c
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

const CREATURE_OR_LAND: Filter = Filter::Or(&[Filter::CREATURE, Filter::LAND]);

card!(
    index = index::AWAKENING,
    oracle_id = "5563427d-ffdc-4bea-ad7d-1605894b8a6c",
    scryfall_id = "59612c02-6028-4953-ac4d-aca94b0ce4b9",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Awakening",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::EachPlayer,
        },
        &[Effect::UntapAll {
            filter: &CREATURE_OR_LAND,
        }],
    )],
);
