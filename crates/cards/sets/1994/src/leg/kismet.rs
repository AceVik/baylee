//! Kismet — {3}{W} — Enchantment
//! Oracle: Artifacts, creatures, and lands your opponents control enter tapped.
//! Set: ME4 #17 — Masters Edition IV | Scryfall ID: f38cdfb1-0437-4afe-a777-60228b8eba69 | Oracle ID: 81fdd1c4-d43b-4f8b-8712-7c2bf45a3e0b

use baylee_cards_dsl::prelude::*;

card!(
    index = index::KISMET,
    oracle_id = "81fdd1c4-d43b-4f8b-8712-7c2bf45a3e0b",
    scryfall_id = "f38cdfb1-0437-4afe-a777-60228b8eba69",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Kismet",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[AbilityDef::Replacement(ReplacementRule::EnterTapped(
        &Filter::And(&[
            Filter::ControlledByOpponent,
            Filter::Or(&[Filter::ARTIFACT, Filter::CREATURE, Filter::LAND])
        ])
    ))],
);
