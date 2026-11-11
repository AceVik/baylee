//! Unstoppable Slasher — {2}{B} — Creature — Zombie Assassin
//! Oracle: Deathtouch
//! Oracle: Whenever this creature deals combat damage to a player, they lose half their life, rounded up.
//! Oracle: When this creature dies, if it had no counters on it, return it to the battlefield tapped under its owner's control with two stun counters on it.
//! Set: DSK #119 — Duskmourn: House of Horror | Scryfall ID: c78da035-6b5b-4136-9ab6-f622b64fdc54 | Oracle ID: 6662968e-ca9e-454c-be44-24ad10e43d3e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::UNSTOPPABLE_SLASHER,
    oracle_id = "6662968e-ca9e-454c-be44-24ad10e43d3e",
    scryfall_id = "c78da035-6b5b-4136-9ab6-f622b64fdc54",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Unstoppable Slasher",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE, subtypes::creature::ASSASSIN],
        power = Some(2),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
