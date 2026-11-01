//! Metalworker — {3} — Artifact Creature — Construct
//! Oracle: {T}: Reveal any number of artifact cards in your hand. Add {C}{C} for each card revealed this way.
//! Set: UDS #135 — Urza's Destiny | Scryfall ID: 2050d414-71c7-4c42-a1ff-4c04068ba7f2 | Oracle ID: 9ed9accb-9f0c-40fa-aeea-2d1e1c9d1bd1
// NOT SUPPORTED: variable-count hand reveal (reveal any number of artifact cards from your hand and add {C}{C} per card revealed) is unsupported; plays as a 1/2 Artifact Creature — Construct body
// PARTIAL: {3} 1/2 Artifact Creature — Construct body implemented; missing "{T}: Reveal any number of artifact cards in your hand. Add {C}{C} for each card revealed this way" because the DSL has no cost or effect to reveal any number of matching cards from your hand and count them.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::METALWORKER,
    oracle_id = "9ed9accb-9f0c-40fa-aeea-2d1e1c9d1bd1",
    scryfall_id = "2050d414-71c7-4c42-a1ff-4c04068ba7f2",
    coverage = Coverage::Partial(
        "variable-count hand reveal (reveal any number of artifact cards from your hand and add {C}{C} per card revealed) is unsupported; plays as a 1/2 Artifact Creature — Construct body"
    ),
    faces = &[face!(
        name = "Metalworker",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::CONSTRUCT],
        power = Some(1),
        toughness = Some(2),
    ),],
);
