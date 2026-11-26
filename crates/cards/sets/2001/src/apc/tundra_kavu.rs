//! Tundra Kavu — {2}{R} — Creature — Kavu
//! Oracle: {T}: Target land becomes a Plains or an Island until end of turn.
//! Set: APC #71 — Apocalypse | Scryfall ID: fc34e735-ac3c-4954-a4c8-3ed55d811715 | Oracle ID: b0b7d88b-694b-4bbe-888d-05245e7d1c3e
// IMPLEMENTED — tap ability sets target land to Plains or Island until end of turn.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static LAND_CHOICES: &[Effect] = &[
    Effect::ModifyTargets {
        modifier: Modifier::SetLandType(subtypes::land::PLAINS),
        duration: Duration::UntilEndOfTurn,
    },
    Effect::ModifyTargets {
        modifier: Modifier::SetLandType(subtypes::land::ISLAND),
        duration: Duration::UntilEndOfTurn,
    },
];

card!(
    index = index::TUNDRA_KAVU,
    oracle_id = "b0b7d88b-694b-4bbe-888d-05245e7d1c3e",
    scryfall_id = "fc34e735-ac3c-4954-a4c8-3ed55d811715",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Tundra Kavu",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::KAVU],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[activated!(
        Cost::TAP,
        &[Effect::ChooseOne {
            choices: LAND_CHOICES
        }],
        target = Some(TargetSpec::Object(&Filter::LAND)),
    )],
);
