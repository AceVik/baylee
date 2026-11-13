//! Sentinel — {4} — Artifact Creature — Shapeshifter
//! Oracle: {0}: This creature's base toughness becomes equal to 1 plus the power of target creature blocking or blocked by this creature. (This effect lasts indefinitely.)
//! Set: CHR #107 — Chronicles | Scryfall ID: ffd4921f-cda5-4318-837d-a3fe4f0d9362 | Oracle ID: c30e8efb-d097-4388-8d5c-1037ccc29fba

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SENTINEL,
    oracle_id = "c30e8efb-d097-4388-8d5c-1037ccc29fba",
    scryfall_id = "ffd4921f-cda5-4318-837d-a3fe4f0d9362",
    faces = &[face!(
        name = "Sentinel",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::SHAPESHIFTER],
        power = Some(1),
        toughness = Some(1),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[activated!(
        cost!("{0}"),
        &[Effect::SetSourceToughness {
            base: Amount::TargetPower,
            offset: 1,
            duration: Duration::Indefinitely
        }],
        target = Some(TargetSpec::Object(&Filter::And(&[
            Filter::CREATURE,
            Filter::Or(&[Filter::BlockingSource, Filter::BlockedBySource])
        ])))
    ),],
);
