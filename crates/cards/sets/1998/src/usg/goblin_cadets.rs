//! Goblin Cadets — {R} — Creature — Goblin
//! Oracle: Whenever this creature blocks or becomes blocked, target opponent gains control of it. (This removes this creature from combat.)
//! Set: USG #189 — Urza's Saga | Scryfall ID: 60081115-16bc-4924-b76d-7cfc0ad2287c | Oracle ID: 32dc847a-d8e9-4bc5-aa73-dfe69ff330a2
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GOBLIN_CADETS,
    oracle_id = "32dc847a-d8e9-4bc5-aa73-dfe69ff330a2",
    scryfall_id = "60081115-16bc-4924-b76d-7cfc0ad2287c",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Goblin Cadets",
        mana_cost = mana!("{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN],
        power = Some(2),
        toughness = Some(1),
    ),],
    abilities = &[triggered!(
        Trigger::BlocksOrBecomesBlockedByOneOrMore(&Filter::CREATURE),
        &[Effect::GainControlAll {
            filter: &Filter::This,
            to: PlayerRel::TargetPlayerOrController,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyOpponent)),
    )],
);
