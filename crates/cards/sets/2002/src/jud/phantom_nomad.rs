//! Phantom Nomad — {1}{W} — Creature — Spirit Nomad
//! Oracle: This creature enters with two +1/+1 counters on it.
//! Oracle: If damage would be dealt to this creature, prevent that damage. Remove a +1/+1 counter from this creature.
//! Set: VMA #38 — Vintage Masters | Scryfall ID: 1f43bb08-0a69-48e2-ac7a-5db92e60337b | Oracle ID: bf65a7a7-a590-41eb-9844-184e5d63e32a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PHANTOM_NOMAD,
    oracle_id = "bf65a7a7-a590-41eb-9844-184e5d63e32a",
    scryfall_id = "1f43bb08-0a69-48e2-ac7a-5db92e60337b",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Phantom Nomad",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SPIRIT, subtypes::creature::NOMAD],
        power = Some(0),
        toughness = Some(0),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
