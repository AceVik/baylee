//! Lava Runner — {1}{R}{R} — Creature — Lizard
//! Oracle: Haste
//! Oracle: Whenever this creature becomes the target of a spell or ability, that spell or ability's controller sacrifices a land of their choice.
//! Set: MMQ #200 — Mercadian Masques | Scryfall ID: 09d0fbe6-6ce1-4b95-afb7-a7386b5033cf | Oracle ID: 48a77c0c-a179-47f2-83b2-da80ba5448f9
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LAVA_RUNNER,
    oracle_id = "48a77c0c-a179-47f2-83b2-da80ba5448f9",
    scryfall_id = "09d0fbe6-6ce1-4b95-afb7-a7386b5033cf",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    keywords = KeywordSet::HASTE,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Lava Runner",
        mana_cost = mana!("{1}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::LIZARD],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[triggered!(
        Trigger::BecomesTarget,
        &[Effect::SacrificeFilter {
            who: PlayerRel::ControllerOfEvent,
            filter: &Filter::LAND,
        }],
    ),],
);
