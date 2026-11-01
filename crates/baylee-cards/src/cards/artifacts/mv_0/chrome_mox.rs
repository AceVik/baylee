//! Chrome Mox — {0} — Artifact
//! Oracle: Imprint — When this artifact enters, you may exile a nonartifact, nonland card from your hand.
//! Oracle: {T}: Add one mana of any of the exiled card's colors.
//! Set: 2XM #240 — Double Masters | Scryfall ID: f340cbf7-5bbe-45b9-a4bf-d1caa500ff93 | Oracle ID: ec3d4466-547c-4e02-b1b5-a156ec4637e9
// IMPLEMENTED — optional imprint and mana of the exiled card's colors.

use baylee_cards_dsl::prelude::*;

static IMPRINT: Filter = Filter::And(&[Filter::Not(&Filter::ARTIFACT), Filter::Not(&Filter::LAND)]);

card!(
    index = index::CHROME_MOX,
    oracle_id = "ec3d4466-547c-4e02-b1b5-a156ec4637e9",
    scryfall_id = "f340cbf7-5bbe-45b9-a4bf-d1caa500ff93",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Chrome Mox",
        mana_cost = mana!("{0}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[
        triggered!(
            Trigger::ETB,
            &[Effect::MayDo {
                effects: &[Effect::ExileChosenFromHandWithSource(&IMPRINT)],
            }],
        ),
        mana_ability!(&[Effect::mana_exiled_colors()]),
    ],
);
