//! Desolation Giant — {2}{R}{R} — Creature — Giant
//! Oracle: Kicker {W}{W} (You may pay an additional {W}{W} as you cast this spell.)
//! Oracle: When this creature enters, destroy all other creatures you control. If it was kicked, destroy all other creatures instead.
//! Set: CM2 #91 — Commander Anthology Volume II | Scryfall ID: b74a47b5-100d-424f-8432-7668c50458f8 | Oracle ID: c23aaf6d-151e-481a-a345-ca00c14940d1
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DESOLATION_GIANT,
    oracle_id = "c23aaf6d-151e-481a-a345-ca00c14940d1",
    scryfall_id = "b74a47b5-100d-424f-8432-7668c50458f8",
    color_identity = ColorSet::from_slice(&[Color::Red, Color::White]),
    faces = &[face!(
        name = "Desolation Giant",
        mana_cost = mana!("{2}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GIANT],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
