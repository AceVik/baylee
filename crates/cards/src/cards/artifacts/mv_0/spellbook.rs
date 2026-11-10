//! Spellbook — {0} — Artifact — Book
//! Oracle: You have no maximum hand size.
//! Set: M10 #220 — Magic 2010 | Scryfall ID: 26f4d2b1-31d7-40d8-bd4e-cdd128076d0f | Oracle ID: 83133f51-bfbd-4db5-9be0-f660e3b66435
// IMPLEMENTED — all printed abilities are represented.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SPELLBOOK,
    oracle_id = "83133f51-bfbd-4db5-9be0-f660e3b66435",
    scryfall_id = "26f4d2b1-31d7-40d8-bd4e-cdd128076d0f",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Spellbook",
        mana_cost = mana!("{0}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[subtypes::artifact::BOOK],
    ),],
    abilities = &[static_ability!(Filter::Any, Modifier::NoMaxHandSize)],
);
