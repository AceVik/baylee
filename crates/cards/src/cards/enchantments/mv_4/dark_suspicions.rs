//! Dark Suspicions — {2}{B}{B} — Enchantment
//! Oracle: At the beginning of each opponent's upkeep, that player loses X life, where X is the number of cards in that player's hand minus the number of cards in your hand.
//! Set: PLS #40 — Planeshift | Scryfall ID: d518e2fd-7767-43d7-92e3-62a4a465154c | Oracle ID: cf5cf174-d976-4cd8-9721-c30fe784fb70
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DARK_SUSPICIONS,
    oracle_id = "cf5cf174-d976-4cd8-9721-c30fe784fb70",
    scryfall_id = "d518e2fd-7767-43d7-92e3-62a4a465154c",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Dark Suspicions",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
