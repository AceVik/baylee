//! Fog — {G} — Instant
//! Oracle: Prevent all combat damage that would be dealt this turn.
//! Set: EMA #167 — Eternal Masters | Scryfall ID: bbc3152e-7b3b-4ac6-8b33-abfebde216aa | Oracle ID: 27e9db49-7af7-4bef-ad4c-bf5dfb92030d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FOG,
    oracle_id = "27e9db49-7af7-4bef-ad4c-bf5dfb92030d",
    scryfall_id = "bbc3152e-7b3b-4ac6-8b33-abfebde216aa",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Fog",
        mana_cost = mana!("{G}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
