//! Avoid Fate — {G} — Instant
//! Oracle: Counter target instant or Aura spell that targets a permanent you control.
//! Set: TSB #73 — Time Spiral Timeshifted | Scryfall ID: 04c9f48d-2213-4f95-8eac-b23bd2d95e34 | Oracle ID: c04dd88f-fb7f-43be-b586-7fc5642073dc

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::AVOID_FATE,
    oracle_id = "c04dd88f-fb7f-43be-b586-7fc5642073dc",
    scryfall_id = "04c9f48d-2213-4f95-8eac-b23bd2d95e34",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Avoid Fate",
        mana_cost = mana!("{G}"),
        types = TypeSet::INSTANT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[Effect::CounterTargetSpell],
        targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::And(&[
            Filter::Or(&[
                Filter::HasType(TypeSet::INSTANT),
                Filter::HasSubtype(baylee_core::generated::subtypes::enchantment::AURA)
            ]),
            Filter::TargetsPermanentYouControl
        ]))))
    ),],
);
