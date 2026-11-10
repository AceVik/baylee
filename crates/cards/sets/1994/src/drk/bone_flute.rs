//! Bone Flute — {3} — Artifact
//! Oracle: {2}, {T}: All creatures get -1/-0 until end of turn.
//! Set: ME3 #192 — Masters Edition III | Scryfall ID: 12aa4c51-4b3d-48c2-bebb-13ea98ebc3b2 | Oracle ID: 8664a004-d786-4357-8856-2f8f5a9ae24d

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BONE_FLUTE,
    oracle_id = "8664a004-d786-4357-8856-2f8f5a9ae24d",
    scryfall_id = "12aa4c51-4b3d-48c2-bebb-13ea98ebc3b2",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Bone Flute",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{2}", TapSelf),
        &[Effect::CreateContinuousEffect {
            layer: Layer::PtModify,
            filter: &Filter::CREATURE,
            modifier: Modifier::ModifyPT(-1, 0),
            duration: Duration::UntilEndOfTurn,
        }],
    )],
);
