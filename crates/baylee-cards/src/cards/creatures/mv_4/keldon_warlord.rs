//! Keldon Warlord — {2}{R}{R} — Creature — Human Barbarian
//! Oracle: Keldon Warlord's power and toughness are each equal to the number of non-Wall creatures you control.
//! Set: ME1 #101 — Masters Edition | Scryfall ID: 8e67a472-3058-47a7-90f5-1d26c4287cab | Oracle ID: acc869f8-dcbe-4d57-baa0-3eef4aceb251
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::KELDON_WARLORD,
    oracle_id = "acc869f8-dcbe-4d57-baa0-3eef4aceb251",
    scryfall_id = "8e67a472-3058-47a7-90f5-1d26c4287cab",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Keldon Warlord",
        mana_cost = mana!("{2}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::BARBARIAN],
        power = Some(0),
        toughness = Some(0),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
