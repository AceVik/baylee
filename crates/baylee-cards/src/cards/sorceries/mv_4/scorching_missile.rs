//! Scorching Missile — {3}{R} — Sorcery
//! Oracle: Scorching Missile deals 4 damage to target player or planeswalker.
//! Oracle: Flashback {9}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
//! Set: ODY #219 — Odyssey | Scryfall ID: 0672960b-4cb5-4ed6-ba3c-6b97290e0330 | Oracle ID: 02e2dbde-2373-4f29-bd9d-787f1805b061
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SCORCHING_MISSILE,
    oracle_id = "02e2dbde-2373-4f29-bd9d-787f1805b061",
    scryfall_id = "0672960b-4cb5-4ed6-ba3c-6b97290e0330",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Scorching Missile",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
