//! Phyrexian Scuta — {3}{B} — Creature — Phyrexian Zombie
//! Oracle: Kicker—Pay 3 life. (You may pay 3 life in addition to any other costs as you cast this spell.)
//! Oracle: If this creature was kicked, it enters with two +1/+1 counters on it.
//! Set: DMR #100 — Dominaria Remastered | Scryfall ID: 5b5b3b5c-121a-406e-a31a-537991309399 | Oracle ID: af2845cf-8770-4efa-8a83-bf9eb4122b78
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PHYREXIAN_SCUTA,
    oracle_id = "af2845cf-8770-4efa-8a83-bf9eb4122b78",
    scryfall_id = "5b5b3b5c-121a-406e-a31a-537991309399",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Phyrexian Scuta",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::PHYREXIAN, subtypes::creature::ZOMBIE],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
