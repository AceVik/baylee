//! Virtue of Knowledge // Vantress Visions — {4}{U} — Enchantment // Instant — Adventure
//! Oracle: If a permanent entering causes a triggered ability of a permanent you control to trigger, that ability triggers an additional time.
//! Oracle: Copy target activated or triggered ability you control. You may choose new targets for the copy.
//! Set: WOE #76 — Wilds of Eldraine | Scryfall ID: df606cf5-67dc-46f4-8c79-1d2f1d054391 | Oracle ID: f0bbcabf-29e7-4c7e-893f-86b64d3620a9
//! Face: Virtue of Knowledge — {4}{U} — Enchantment
//! Face: Vantress Visions — {1}{U} — Instant — Adventure
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::VIRTUE_OF_KNOWLEDGE,
    oracle_id = "f0bbcabf-29e7-4c7e-893f-86b64d3620a9",
    scryfall_id = "df606cf5-67dc-46f4-8c79-1d2f1d054391",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[
        face!(
            name = "Virtue of Knowledge",
            mana_cost = mana!("{4}{U}"),
            types = TypeSet::ENCHANTMENT,
        ),
        face!(
            name = "Vantress Visions",
            mana_cost = mana!("{1}{U}"),
            types = TypeSet::INSTANT,
            subtypes = &[subtypes::spell::ADVENTURE],
        ),
    ],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
