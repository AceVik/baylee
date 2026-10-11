//! Alchor's Tomb — {4} — Artifact
//! Oracle: {2}, {T}: Target permanent you control becomes the color of your choice. (This effect lasts indefinitely.)
//! Set: ME4 #178 — Masters Edition IV | Scryfall ID: e10ad2c6-b401-4a4e-bdce-705e2b87f492 | Oracle ID: 61473d8e-45f1-4753-918d-04918a466031

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ALCHOR_S_TOMB,
    oracle_id = "61473d8e-45f1-4753-918d-04918a466031",
    scryfall_id = "e10ad2c6-b401-4a4e-bdce-705e2b87f492",
    faces = &[face!(
        name = "Alchor's Tomb",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[activated!(
        cost!("{2}", TapSelf),
        &[Effect::ChooseColors {
            subject: ColorSubject::Target,
            max: 1,
            duration: Duration::Indefinitely,
        }],
        target = Some(TargetSpec::Object(&Filter::ControlledByYou))
    ),],
);
