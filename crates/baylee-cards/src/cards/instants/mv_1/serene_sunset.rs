//! Serene Sunset — {X}{G} — Instant
//! Oracle: Prevent all combat damage X target creatures would deal this turn.
//! Set: JUD #131 — Judgment | Scryfall ID: 0a6ded26-b748-406d-8740-9b8590be2bb1 | Oracle ID: e2ef027d-0ef7-4c8b-a594-3918290b02d9
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SERENE_SUNSET,
    oracle_id = "e2ef027d-0ef7-4c8b-a594-3918290b02d9",
    scryfall_id = "0a6ded26-b748-406d-8740-9b8590be2bb1",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Serene Sunset",
        mana_cost = mana!("{X}{G}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
