//! Skulking Ghost — {1}{B} — Creature — Spirit
//! Oracle: Flying
//! Oracle: When this creature becomes the target of a spell or ability, sacrifice it.
//! Set: EMA #107 — Eternal Masters | Scryfall ID: 285f37a0-88ad-4cd6-8a8f-60c2d9805437 | Oracle ID: cd97f8b4-e751-4b52-987b-a442aef57809

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SKULKING_GHOST,
    oracle_id = "cd97f8b4-e751-4b52-987b-a442aef57809",
    scryfall_id = "285f37a0-88ad-4cd6-8a8f-60c2d9805437",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Skulking Ghost",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SPIRIT],
        power = Some(2),
        toughness = Some(1),
    ),],
    abilities = &[triggered!(Trigger::BecomesTarget, &[Effect::SacrificeSelf])],
);
