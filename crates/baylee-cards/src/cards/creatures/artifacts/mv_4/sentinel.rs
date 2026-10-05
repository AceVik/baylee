//! Sentinel — {4} — Artifact Creature — Shapeshifter
//! Oracle: {0}: This creature's base toughness becomes equal to 1 plus the power of target creature blocking or blocked by this creature. (This effect lasts indefinitely.)
//! Set: CHR #107 — Chronicles | Scryfall ID: ffd4921f-cda5-4318-837d-a3fe4f0d9362 | Oracle ID: c30e8efb-d097-4388-8d5c-1037ccc29fba
// PARTIAL — the only ability is off the card.

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
    coverage = Coverage::Partial(
        "no Filter names the creatures blocking or blocked by the source, \
         and SetPTFilter sets both halves of base power and toughness where \
         the sentence changes base toughness alone"
    ),
    // NOT SUPPORTED: "{0}: This creature's base toughness becomes equal to 1
    // plus the power of target creature blocking or blocked by this
    // creature. (This effect lasts indefinitely.)" — no `Filter` names the
    // combat partners of the source (`Filter::Blocking` tests a creature's
    // own combat state), so the ability has no target it can name.
);
