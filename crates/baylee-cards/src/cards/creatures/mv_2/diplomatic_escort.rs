//! Diplomatic Escort — {1}{U} — Creature — Human Spellshaper
//! Oracle: {U}, {T}, Discard a card: Counter target spell or ability that targets a creature.
//! Set: MMQ #74 — Mercadian Masques | Scryfall ID: 9356bdbb-d647-4f51-a7a3-18ecea898a7f | Oracle ID: 55cc14d0-11ed-444d-884a-cfb692ba632b
// PARTIAL — 1/1 Human Spellshaper body.
// NOT SUPPORTED: "{U}, {T}, Discard a card: Counter target spell or ability that targets a creature" — `Filter` has no predicate matching a spell or ability on the stack that targets a creature.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DIPLOMATIC_ESCORT,
    oracle_id = "55cc14d0-11ed-444d-884a-cfb692ba632b",
    scryfall_id = "9356bdbb-d647-4f51-a7a3-18ecea898a7f",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial("counter spell or ability that targets a creature not supported"),
    faces = &[face!(
        name = "Diplomatic Escort",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SPELLSHAPER],
        power = Some(1),
        toughness = Some(1),
    ),],
);
