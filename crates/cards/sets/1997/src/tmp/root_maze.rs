//! Root Maze — {G} — Enchantment
//! Oracle: Artifacts and lands enter tapped.
//! Set: 10E #292 — Tenth Edition | Scryfall ID: 727c757a-2ae5-45fd-9832-646a355a5710 | Oracle ID: 2f4abc5e-5aad-455e-b1ec-137d7ac4a10c
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

static ARTIFACT_OR_LAND: Filter = Filter::Or(&[Filter::ARTIFACT, Filter::LAND]);

card!(
    index = index::ROOT_MAZE,
    oracle_id = "2f4abc5e-5aad-455e-b1ec-137d7ac4a10c",
    scryfall_id = "727c757a-2ae5-45fd-9832-646a355a5710",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Root Maze",
        mana_cost = mana!("{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[AbilityDef::Replacement(ReplacementRule::EnterTapped(
        &ARTIFACT_OR_LAND,
    ))],
);
