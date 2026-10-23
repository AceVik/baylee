//! Chimeric Staff — {4} — Artifact
//! Oracle: {X}: This artifact becomes an X/X Construct artifact creature until end of turn.
//! Set: 10E #313 — Tenth Edition | Scryfall ID: 9ca0425d-f41c-4ba5-ac06-9ce5bab2a4b9 | Oracle ID: 771f5e29-b4d1-4466-8510-ce3f89ad7173
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CHIMERIC_STAFF,
    oracle_id = "771f5e29-b4d1-4466-8510-ce3f89ad7173",
    scryfall_id = "9ca0425d-f41c-4ba5-ac06-9ce5bab2a4b9",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Chimeric Staff",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{X}"),
        &[
            Effect::continuous(
                &Filter::This,
                Modifier::AddType(TypeSet::CREATURE),
                Duration::UntilEndOfTurn,
            ),
            Effect::continuous(
                &Filter::This,
                Modifier::AddType(TypeSet::ARTIFACT),
                Duration::UntilEndOfTurn,
            ),
            Effect::continuous(
                &Filter::This,
                Modifier::ReplaceCreatureTypes(subtypes::creature::CONSTRUCT),
                Duration::UntilEndOfTurn,
            ),
            Effect::SetPTFilter {
                filter: &Filter::This,
                power: Amount::X,
                toughness: Amount::X,
                duration: Duration::UntilEndOfTurn,
            },
        ],
    )],
);
