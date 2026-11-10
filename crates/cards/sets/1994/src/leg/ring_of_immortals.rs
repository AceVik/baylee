//! Ring of Immortals — {5} — Artifact
//! Oracle: {3}, {T}: Counter target instant or Aura spell that targets a permanent you control.
//! Set: LEG #293 — Legends | Scryfall ID: 61706102-67fd-4167-bd7d-ec6da41db362 | Oracle ID: 468a8c2e-0cc2-4108-809d-42ca1eb25ff2

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RING_OF_IMMORTALS,
    oracle_id = "468a8c2e-0cc2-4108-809d-42ca1eb25ff2",
    scryfall_id = "61706102-67fd-4167-bd7d-ec6da41db362",
    faces = &[face!(
        name = "Ring of Immortals",
        mana_cost = mana!("{5}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[activated!(
        cost!("{3}", TapSelf),
        &[Effect::CounterTargetSpell],
        target = Some(TargetSpec::Spell(&Filter::And(&[
            Filter::Or(&[
                Filter::HasType(TypeSet::INSTANT),
                Filter::HasSubtype(baylee_core::generated::subtypes::enchantment::AURA)
            ]),
            Filter::TargetsPermanentYouControl
        ])))
    ),],
);
