//! Millikin — {2} — Artifact Creature — Construct
//! Oracle: {T}, Mill a card: Add {C}. (Activate only as an instant. To mill a card, put the top card of your library into your graveyard.)
//! Set: SOC #351 — Secrets of Strixhaven Commander | Scryfall ID: c2073453-4108-4039-a8c2-0f94dd7bf237 | Oracle ID: fa4dffda-6f04-4d0b-829d-28a1a5794dee
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MILLIKIN,
    oracle_id = "fa4dffda-6f04-4d0b-829d-28a1a5794dee",
    scryfall_id = "c2073453-4108-4039-a8c2-0f94dd7bf237",
    faces = &[face!(
        name = "Millikin",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::CONSTRUCT],
        power = Some(0),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
