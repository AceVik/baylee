//! Far Wanderings — {2}{G} — Sorcery
//! Oracle: Search your library for a basic land card, put that card onto the battlefield tapped, then shuffle.
//! Oracle: Threshold — If there are seven or more cards in your graveyard, instead search your library for up to three basic land cards, put them onto the battlefield tapped, then shuffle.
//! Set: ZNC #66 — Zendikar Rising Commander | Scryfall ID: db67c59e-a9fb-4580-a8cc-ac5ef6c07440 | Oracle ID: 2ef797ef-c05f-4309-85c7-1bd151dac3fa
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FAR_WANDERINGS,
    oracle_id = "2ef797ef-c05f-4309-85c7-1bd151dac3fa",
    scryfall_id = "db67c59e-a9fb-4580-a8cc-ac5ef6c07440",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Far Wanderings",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
