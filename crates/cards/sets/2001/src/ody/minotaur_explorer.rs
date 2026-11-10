//! Minotaur Explorer — {1}{R} — Creature — Minotaur Scout
//! Oracle: When this creature enters, sacrifice it unless you discard a card at random.
//! Set: ODY #206 — Odyssey | Scryfall ID: 55725e38-d60a-41a2-93b0-2eefe6d2cc59 | Oracle ID: 22374ec2-61b3-46eb-82ed-93bbb42403c4
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MINOTAUR_EXPLORER,
    oracle_id = "22374ec2-61b3-46eb-82ed-93bbb42403c4",
    scryfall_id = "55725e38-d60a-41a2-93b0-2eefe6d2cc59",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Minotaur Explorer",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::MINOTAUR, subtypes::creature::SCOUT],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
