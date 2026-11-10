//! Metamorphose — {1}{U} — Instant
//! Oracle: Put target permanent an opponent controls on top of its owner's library. That opponent may put an artifact, creature, enchantment, or land card from their hand onto the battlefield.
//! Set: SCG #40 — Scourge | Scryfall ID: a0f0c20c-184e-4d27-ae8b-933abb6fee0c | Oracle ID: 77d6c141-5670-4120-9346-dc2fdcb66e50
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::METAMORPHOSE,
    oracle_id = "77d6c141-5670-4120-9346-dc2fdcb66e50",
    scryfall_id = "a0f0c20c-184e-4d27-ae8b-933abb6fee0c",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Metamorphose",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
