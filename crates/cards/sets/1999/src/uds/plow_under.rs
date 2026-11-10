//! Plow Under — {3}{G}{G} — Sorcery
//! Oracle: Put two target lands on top of their owners' libraries.
//! Set: 8ED #272 — Eighth Edition | Scryfall ID: 27128f26-0d89-4962-96e8-5fa343031a26 | Oracle ID: 23b36aef-961f-4a78-9652-8594ad73102e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PLOW_UNDER,
    oracle_id = "23b36aef-961f-4a78-9652-8594ad73102e",
    scryfall_id = "27128f26-0d89-4962-96e8-5fa343031a26",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Plow Under",
        mana_cost = mana!("{3}{G}{G}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
