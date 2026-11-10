//! Mystic Compass — {2} — Artifact
//! Oracle: {1}, {T}: Target land becomes the basic land type of your choice until end of turn.
//! Set: 6ED #302 — Classic Sixth Edition | Scryfall ID: e80bfd76-52f9-4dea-9056-8570c9b290a4 | Oracle ID: 87bafa97-38fd-41a2-8b1d-315e88d9d0bf
// IMPLEMENTED — {1}, {T} activated ability setting target land's basic land type to the chosen type until end of turn.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

static TYPES: &[Effect] = &[
    Effect::ModifyTargets {
        modifier: Modifier::SetLandType(land::PLAINS),
        duration: Duration::UntilEndOfTurn,
    },
    Effect::ModifyTargets {
        modifier: Modifier::SetLandType(land::ISLAND),
        duration: Duration::UntilEndOfTurn,
    },
    Effect::ModifyTargets {
        modifier: Modifier::SetLandType(land::SWAMP),
        duration: Duration::UntilEndOfTurn,
    },
    Effect::ModifyTargets {
        modifier: Modifier::SetLandType(land::MOUNTAIN),
        duration: Duration::UntilEndOfTurn,
    },
    Effect::ModifyTargets {
        modifier: Modifier::SetLandType(land::FOREST),
        duration: Duration::UntilEndOfTurn,
    },
];

card!(
    index = index::MYSTIC_COMPASS,
    oracle_id = "87bafa97-38fd-41a2-8b1d-315e88d9d0bf",
    scryfall_id = "e80bfd76-52f9-4dea-9056-8570c9b290a4",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Mystic Compass",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{1}", TapSelf),
        &[Effect::ChooseOne { choices: TYPES }],
        target = Some(TargetSpec::Object(&Filter::LAND)),
    )],
);
