//! New Frontiers — {X}{G} — Sorcery
//! Oracle: Each player may search their library for up to X basic land cards and put them onto the battlefield tapped. Then each player who searched their library this way shuffles.
//! Set: ODY #257 — Odyssey | Scryfall ID: fd1f6cfa-9576-4bb8-83db-33c2b147206d | Oracle ID: 71362163-2ac3-4f8b-86b9-34fe083e2095
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::NEW_FRONTIERS,
    oracle_id = "71362163-2ac3-4f8b-86b9-34fe083e2095",
    scryfall_id = "fd1f6cfa-9576-4bb8-83db-33c2b147206d",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "New Frontiers",
        mana_cost = mana!("{X}{G}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
