//! Unstable Frontier — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Target land you control becomes the basic land type of your choice until end of turn.
//! Set: CON #145 — Conflux | Scryfall ID: d97e739f-8675-488e-be2b-4e455fbe390b | Oracle ID: 495214b5-2eab-4fe4-8879-a30a57a67163

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
    index = index::UNSTABLE_FRONTIER,
    oracle_id = "495214b5-2eab-4fe4-8879-a30a57a67163",
    scryfall_id = "d97e739f-8675-488e-be2b-4e455fbe390b",
    faces = &[face!(name = "Unstable Frontier", types = TypeSet::LAND)],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            Cost::TAP,
            &[Effect::ChooseOne { choices: TYPES }],
            target = Some(TargetSpec::Object(&Filter::YOUR_LAND))
        ),
    ],
);
