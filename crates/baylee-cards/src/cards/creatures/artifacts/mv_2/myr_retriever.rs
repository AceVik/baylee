//! Myr Retriever — {2} — Artifact Creature — Myr
//! Oracle: When this creature dies, return another target artifact card from your graveyard to your hand.
//! Set: 2XM #277 — Double Masters | Scryfall ID: 7f0149d4-0731-474a-a1c3-28c25e486c14 | Oracle ID: d07d3be3-f69d-4484-8467-cffd43871788
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MYR_RETRIEVER,
    oracle_id = "d07d3be3-f69d-4484-8467-cffd43871788",
    scryfall_id = "7f0149d4-0731-474a-a1c3-28c25e486c14",
    faces = &[face!(
        name = "Myr Retriever",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::MYR],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
