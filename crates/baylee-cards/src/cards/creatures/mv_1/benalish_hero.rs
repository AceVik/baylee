//! Benalish Hero — {W} — Creature — Human Soldier
//! Oracle: Banding (Any creatures with banding, and up to one without, can attack in a band. Bands are blocked as a group. If any creatures with banding you control are blocking or being blocked by a creature, you divide that creature's combat damage, not its controller, among any of the creatures it's being blocked by or is blocking.)
//! Set: ME1 #5 — Masters Edition | Scryfall ID: a14ba3be-f7cb-47e5-b6c4-d09cfd3cac34 | Oracle ID: 4c81cfb7-8765-4e28-ae33-4287fa9a86cc
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BENALISH_HERO,
    oracle_id = "4c81cfb7-8765-4e28-ae33-4287fa9a86cc",
    scryfall_id = "a14ba3be-f7cb-47e5-b6c4-d09cfd3cac34",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Benalish Hero",
        mana_cost = mana!("{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SOLDIER],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
