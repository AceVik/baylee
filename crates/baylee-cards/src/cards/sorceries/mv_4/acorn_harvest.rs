//! Acorn Harvest — {3}{G} — Sorcery
//! Oracle: Create two 1/1 green Squirrel creature tokens.
//! Oracle: Flashback—{1}{G}, Pay 3 life. (You may cast this card from your graveyard for its flashback cost. Then exile it.)
//! Set: TOR #118 — Torment | Scryfall ID: 32987720-cc0c-416b-b79b-217d3b37542d | Oracle ID: 6aeba615-2d37-4eb8-a377-41a517e95aa3
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ACORN_HARVEST,
    oracle_id = "6aeba615-2d37-4eb8-a377-41a517e95aa3",
    scryfall_id = "32987720-cc0c-416b-b79b-217d3b37542d",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Acorn Harvest",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
