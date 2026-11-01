//! Viashino Sandscout — {1}{R} — Creature — Lizard Scout
//! Oracle: Haste (This creature can attack and {T} as soon as it comes under your control.)
//! Oracle: At the beginning of the end step, return this creature to its owner's hand. (Return it only if it's on the battlefield.)
//! Set: 10E #246 — Tenth Edition | Scryfall ID: 102f139d-1baf-4ca6-a940-70b7efdc29f2 | Oracle ID: f8d5a65c-382b-4277-9c08-5dec9e395b02
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::VIASHINO_SANDSCOUT,
    oracle_id = "f8d5a65c-382b-4277-9c08-5dec9e395b02",
    scryfall_id = "102f139d-1baf-4ca6-a940-70b7efdc29f2",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    keywords = KeywordSet::HASTE,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Viashino Sandscout",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::LIZARD, subtypes::creature::SCOUT],
        power = Some(2),
        toughness = Some(1),
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::End,
            whose: PlayerRel::EachPlayer
        },
        &[Effect::bounce(TargetSpec::ThisObject)]
    ),],
);
