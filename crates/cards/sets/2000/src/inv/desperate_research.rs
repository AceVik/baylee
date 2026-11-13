//! Desperate Research — {1}{B} — Sorcery
//! Oracle: Choose a card name other than a basic land card name. Reveal the top seven cards of your library and put all of them with that name into your hand. Exile the rest.
//! Set: INV #100 — Invasion | Scryfall ID: 6a42ac7e-4a27-488c-a2e7-338b18103b02 | Oracle ID: 308b49ad-ebab-41c4-9e09-44202549bafc
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DESPERATE_RESEARCH,
    oracle_id = "308b49ad-ebab-41c4-9e09-44202549bafc",
    scryfall_id = "6a42ac7e-4a27-488c-a2e7-338b18103b02",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Desperate Research",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
