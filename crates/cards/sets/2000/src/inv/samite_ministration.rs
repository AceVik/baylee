//! Samite Ministration — {1}{W} — Instant
//! Oracle: Prevent all damage that would be dealt to you this turn by a source of your choice. Whenever damage from a black or red source is prevented this way this turn, you gain that much life.
//! Set: INV #36 — Invasion | Scryfall ID: b1de62ed-79e6-4daf-a2ab-dc0726e1f7df | Oracle ID: af669df1-3526-46ca-b90c-19b112b7ba44
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SAMITE_MINISTRATION,
    oracle_id = "af669df1-3526-46ca-b90c-19b112b7ba44",
    scryfall_id = "b1de62ed-79e6-4daf-a2ab-dc0726e1f7df",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Samite Ministration",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
