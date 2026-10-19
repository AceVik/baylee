//! Barl's Cage — {4} — Artifact
//! Oracle: {3}: Target creature doesn't untap during its controller's next untap step.
//! Set: ME3 #190 — Masters Edition III | Scryfall ID: c51825dc-9854-4a45-80e9-91924e406586 | Oracle ID: eb260cd3-9060-4a5e-bc60-a8ca13d542cc

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BARL_S_CAGE,
    oracle_id = "eb260cd3-9060-4a5e-bc60-a8ca13d542cc",
    scryfall_id = "c51825dc-9854-4a45-80e9-91924e406586",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Barl's Cage",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{3}"),
        &[Effect::SuppressUntap {
            target: TargetSpec::Object(&Filter::CREATURE),
            steps: 1,
        }],
        target = Some(TargetSpec::Object(&Filter::CREATURE)),
    )],
);
