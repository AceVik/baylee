//! Saprazzan Breaker — {4}{U} — Creature — Beast
//! Oracle: {U}: Mill a card. If a land card was milled this way, this creature can't be blocked this turn.
//! Set: MMQ #98 — Mercadian Masques | Scryfall ID: 2de7bf0f-5ad5-467b-ad80-28517951bbe1 | Oracle ID: f3413f57-229e-49dd-a776-6905b80b2b12
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SAPRAZZAN_BREAKER,
    oracle_id = "f3413f57-229e-49dd-a776-6905b80b2b12",
    scryfall_id = "2de7bf0f-5ad5-467b-ad80-28517951bbe1",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Saprazzan Breaker",
        mana_cost = mana!("{4}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BEAST],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
