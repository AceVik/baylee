//! Plaguebearer — {1}{B} — Creature — Zombie
//! Oracle: {X}{X}{B}: Destroy target nonblack creature with mana value X.
//! Set: EXO #71 — Exodus | Scryfall ID: a8493df6-9954-4e33-867c-ca4bcf3953b2 | Oracle ID: f8cc1dc4-740d-4c5b-ac51-7355730a6595
// IMPLEMENTED — all printed abilities are represented.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const NONBLACK_CREATURE_CMC_X: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Not(&Filter::HasColor(ColorSet::from_slice(&[Color::Black]))),
    Filter::CmcExactlyX,
]);

card!(
    index = index::PLAGUEBEARER,
    oracle_id = "f8cc1dc4-740d-4c5b-ac51-7355730a6595",
    scryfall_id = "a8493df6-9954-4e33-867c-ca4bcf3953b2",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Plaguebearer",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!("{X}{X}{B}"),
        &[Effect::destroy(TargetSpec::Object(
            &NONBLACK_CREATURE_CMC_X
        ))],
        target = Some(TargetSpec::Object(&NONBLACK_CREATURE_CMC_X))
    ),],
);
