//! Golgothian Sylex — {4} — Artifact
//! Oracle: {1}, {T}: Each nontoken permanent with a name originally printed in the Antiquities expansion is sacrificed by its controller.
//! Set: ATQ #51 — Antiquities | Scryfall ID: 856be1dd-a20b-49c2-be9d-7db76c7efd8b | Oracle ID: 95197f8f-c24e-4d13-a106-7186b3cc59e0

use baylee_cards_dsl::prelude::*;

static NONTOKEN_ANTIQUITIES: Filter = Filter::And(&[
    Filter::Not(&Filter::IsToken),
    Filter::NameOriginallyPrintedIn(OriginalExpansion::Antiquities),
]);

card!(
    index = index::GOLGOTHIAN_SYLEX,
    oracle_id = "95197f8f-c24e-4d13-a106-7186b3cc59e0",
    scryfall_id = "856be1dd-a20b-49c2-be9d-7db76c7efd8b",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Golgothian Sylex",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{1}", TapSelf),
        &[Effect::SacrificeAll {
            filter: &NONTOKEN_ANTIQUITIES,
        }],
    )],
);
