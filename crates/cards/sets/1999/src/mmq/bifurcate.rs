//! Bifurcate — {3}{G} — Sorcery
//! Oracle: Search your library for a permanent card with the same name as target nontoken creature, put that card onto the battlefield, then shuffle.
//! Set: MMQ #230 — Mercadian Masques | Scryfall ID: dedb483e-b2c6-46c6-b02b-a49599d33521 | Oracle ID: f21ec455-aaf6-4842-aea2-69a7fd99b80f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BIFURCATE,
    oracle_id = "f21ec455-aaf6-4842-aea2-69a7fd99b80f",
    scryfall_id = "dedb483e-b2c6-46c6-b02b-a49599d33521",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Bifurcate",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
