//! Pyre Zombie — {1}{B}{R} — Creature — Zombie
//! Oracle: At the beginning of your upkeep, if this card is in your graveyard, you may pay {1}{B}{B}. If you do, return it to your hand.
//! Oracle: {1}{R}{R}, Sacrifice this creature: It deals 2 damage to any target.
//! Set: DMR #194 — Dominaria Remastered | Scryfall ID: 49c2db45-0ef7-45c8-ac65-eb785c173bd7 | Oracle ID: accb82be-4f90-4d9a-bafd-f8de3622b3a3
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PYRE_ZOMBIE,
    oracle_id = "accb82be-4f90-4d9a-bafd-f8de3622b3a3",
    scryfall_id = "49c2db45-0ef7-45c8-ac65-eb785c173bd7",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    faces = &[face!(
        name = "Pyre Zombie",
        mana_cost = mana!("{1}{B}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE],
        power = Some(2),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
