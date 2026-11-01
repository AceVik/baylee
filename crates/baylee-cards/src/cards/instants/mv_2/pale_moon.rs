//! Pale Moon — {1}{U} — Instant
//! Oracle: Until end of turn, if a player taps a nonbasic land for mana, it produces colorless mana instead of any other type.
//! Set: NEM #36 — Nemesis | Scryfall ID: aeb282bb-d0b8-4822-8197-ff0523549309 | Oracle ID: 78e3a3c3-3091-46ab-a47a-86b7db68ced4
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PALE_MOON,
    oracle_id = "78e3a3c3-3091-46ab-a47a-86b7db68ced4",
    scryfall_id = "aeb282bb-d0b8-4822-8197-ff0523549309",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Pale Moon",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
