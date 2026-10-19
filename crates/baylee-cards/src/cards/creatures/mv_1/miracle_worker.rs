//! Miracle Worker — {W} — Creature — Human Cleric
//! Oracle: {T}: Destroy target Aura attached to a creature you control.
//! Set: DRK #13 — The Dark | Scryfall ID: 35d29bda-096c-44d4-b45e-c2c507f8efbe | Oracle ID: 86ddbc56-3318-49cf-be95-78772ef87637

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const AURA_ON_YOUR_CREATURE: Filter = Filter::And(&[
    Filter::HasSubtype(subtypes::enchantment::AURA),
    Filter::AttachedTo(&Filter::YOUR_CREATURE),
]);

card!(
    index = index::MIRACLE_WORKER,
    oracle_id = "86ddbc56-3318-49cf-be95-78772ef87637",
    scryfall_id = "35d29bda-096c-44d4-b45e-c2c507f8efbe",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Miracle Worker",
        mana_cost = mana!("{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(1),
        toughness = Some(1),
        abilities = &[activated!(
            Cost::TAP,
            &[Effect::destroy(TargetSpec::Object(&AURA_ON_YOUR_CREATURE))],
            target = Some(TargetSpec::Object(&AURA_ON_YOUR_CREATURE)),
        )],
    ),],
);
