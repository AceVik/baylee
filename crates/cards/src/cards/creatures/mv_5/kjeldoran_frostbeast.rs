//! Kjeldoran Frostbeast — {3}{G}{W} — Creature — Elemental Beast
//! Oracle: At end of combat, destroy all creatures blocking or blocked by this creature.
//! Set: ME3 #156 — Masters Edition III | Scryfall ID: 4d923d51-a6c8-416e-94cf-cac785d29d7d | Oracle ID: 39050823-ccda-43c0-818d-d5db15d82b99

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const BLOCKING_OR_BLOCKED_BY_SOURCE: Filter =
    Filter::Or(&[Filter::BlockingSource, Filter::BlockedBySource]);

card!(
    index = index::KJELDORAN_FROSTBEAST,
    oracle_id = "39050823-ccda-43c0-818d-d5db15d82b99",
    scryfall_id = "4d923d51-a6c8-416e-94cf-cac785d29d7d",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Kjeldoran Frostbeast",
        mana_cost = mana!("{3}{G}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELEMENTAL, subtypes::creature::BEAST],
        power = Some(2),
        toughness = Some(4),
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::CombatEnd,
            whose: PlayerRel::EachPlayer,
        },
        &[Effect::destroy_all(&BLOCKING_OR_BLOCKED_BY_SOURCE)],
    )],
);
