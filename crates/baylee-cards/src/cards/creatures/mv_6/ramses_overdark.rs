//! Ramses Overdark — {2}{U}{U}{B}{B} — Legendary Creature — Human Assassin
//! Oracle: {T}: Destroy target enchanted creature.
//! Set: ME3 #169 — Masters Edition III | Scryfall ID: 61bb42b2-327b-47d3-9fe3-76c029d91ee9 | Oracle ID: 6c9a9071-8159-4607-9465-de796f1dc2cc

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RAMSES_OVERDARK,
    oracle_id = "6c9a9071-8159-4607-9465-de796f1dc2cc",
    scryfall_id = "61bb42b2-327b-47d3-9fe3-76c029d91ee9",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Ramses Overdark",
        mana_cost = mana!("{2}{U}{U}{B}{B}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::ASSASSIN],
        power = Some(4),
        toughness = Some(3),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[activated!(
        cost!(TapSelf),
        &[Effect::destroy(TargetSpec::Object(&Filter::And(&[
            Filter::CREATURE,
            Filter::Enchanted,
        ])))],
        target = Some(TargetSpec::Object(&Filter::And(&[
            Filter::CREATURE,
            Filter::Enchanted
        ])))
    )],
);
