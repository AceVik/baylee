//! Phyrexian Grimoire — {3} — Artifact — Book
//! Oracle: {4}, {T}: Target opponent chooses one of the top two cards of your graveyard. Exile that card and put the other one into your hand.
//! Set: TMP #301 — Tempest | Scryfall ID: 2fcf5b8a-563b-48d7-a874-e9c226192320 | Oracle ID: 090052a8-9e5b-4024-881f-5f8b65db7aa7
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PHYREXIAN_GRIMOIRE,
    oracle_id = "090052a8-9e5b-4024-881f-5f8b65db7aa7",
    scryfall_id = "2fcf5b8a-563b-48d7-a874-e9c226192320",
    faces = &[face!(
        name = "Phyrexian Grimoire",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[subtypes::artifact::BOOK],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
