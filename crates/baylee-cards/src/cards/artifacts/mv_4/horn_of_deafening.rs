//! Horn of Deafening — {4} — Artifact
//! Oracle: {2}, {T}: Prevent all combat damage that would be dealt by target creature this turn.
//! Set: ME4 #205 — Masters Edition IV | Scryfall ID: bfbd36dd-3e6f-40c7-a166-0e8ecaabe7fa | Oracle ID: 50a1c14a-003f-424b-bb8e-2e2d51465a90
// IMPLEMENTED — {2}, {T}: prevent all combat damage from target creature
// until end of turn.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HORN_OF_DEAFENING,
    oracle_id = "50a1c14a-003f-424b-bb8e-2e2d51465a90",
    scryfall_id = "bfbd36dd-3e6f-40c7-a166-0e8ecaabe7fa",
    faces = &[face!(
        name = "Horn of Deafening",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[activated!(
        cost!("{2}", TapSelf),
        &[Effect::continuous(
            &Filter::This,
            Modifier::PreventDamageFromIt,
            Duration::UntilEndOfTurn
        )],
        target = Some(TargetSpec::Object(&Filter::CREATURE))
    )],
);
