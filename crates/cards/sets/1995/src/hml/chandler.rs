//! Chandler — {4}{R} — Legendary Creature — Human Rogue
//! Oracle: {R}{R}{R}, {T}: Destroy target artifact creature.
//! Set: HML #69 — Homelands | Scryfall ID: 4dd3a8e3-9a90-44f4-996c-57242d3c47a5 | Oracle ID: 520abf49-7631-4e7f-889e-547ccc8c3b95
// IMPLEMENTED — {R}{R}{R}, {T}: Destroy target artifact creature.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ARTIFACT_CREATURE: Filter = Filter::And(&[Filter::ARTIFACT, Filter::CREATURE]);

card!(
    index = index::CHANDLER,
    oracle_id = "520abf49-7631-4e7f-889e-547ccc8c3b95",
    scryfall_id = "4dd3a8e3-9a90-44f4-996c-57242d3c47a5",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    commander = CommanderRule::Legendary,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Chandler",
        mana_cost = mana!("{4}{R}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::ROGUE],
        power = Some(3),
        toughness = Some(3),
    ),],
    abilities = &[activated!(
        cost!("{R}{R}{R}", TapSelf),
        &[Effect::destroy(TargetSpec::Object(&ARTIFACT_CREATURE))],
        targets = Some(TargetReq::one(TargetSpec::Object(&ARTIFACT_CREATURE)))
    )],
);
