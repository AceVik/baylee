//! Talruum Piper — {4}{R} — Creature — Minotaur
//! Oracle: All creatures with flying able to block this creature do so.
//! Set: VIS #98 — Visions | Scryfall ID: ca2cb9a7-5063-4b31-9782-8bfd784bca0a | Oracle ID: d18e60ed-ab6e-4b73-88f6-caa3f8e9c78e
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const CREATURE_WITH_FLYING: Filter =
    Filter::And(&[Filter::CREATURE, Filter::HasKeyword(KeywordSet::FLYING)]);

card!(
    index = index::TALRUUM_PIPER,
    oracle_id = "d18e60ed-ab6e-4b73-88f6-caa3f8e9c78e",
    scryfall_id = "ca2cb9a7-5063-4b31-9782-8bfd784bca0a",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Talruum Piper",
        mana_cost = mana!("{4}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::MINOTAUR],
        power = Some(3),
        toughness = Some(3),
    ),],
    abilities = &[static_ability!(
        Filter::This,
        Modifier::MustBeBlockedByAllAbleMatching(&CREATURE_WITH_FLYING),
    )],
);
