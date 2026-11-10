//! Whispergear Sneak — {1} — Artifact Creature — Construct
//! Oracle: Draft this card face up.
//! Oracle: During the draft, you may turn this card face down. If you do, look at any unopened booster pack in the draft or any booster pack not being looked at by another player.
//! Set: CNS #64 — Conspiracy | Scryfall ID: e36074e7-d173-44a1-bde3-9a89293467b1 | Oracle ID: e40daee2-f9e6-489f-b3dd-274e5a6b8604
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WHISPERGEAR_SNEAK,
    oracle_id = "e40daee2-f9e6-489f-b3dd-274e5a6b8604",
    scryfall_id = "e36074e7-d173-44a1-bde3-9a89293467b1",
    faces = &[face!(
        name = "Whispergear Sneak",
        mana_cost = mana!("{1}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::CONSTRUCT],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
