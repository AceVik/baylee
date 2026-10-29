//! Cetavolver — {1}{U} — Creature — Volver
//! Oracle: Kicker {1}{R} and/or {G} (You may pay an additional {1}{R} and/or {G} as you cast this spell.)
//! Oracle: If this creature was kicked with its {1}{R} kicker, it enters with two +1/+1 counters on it and with first strike.
//! Oracle: If this creature was kicked with its {G} kicker, it enters with a +1/+1 counter on it and with trample.
//! Set: APC #21 — Apocalypse | Scryfall ID: 69063cc2-4f6e-4cce-bb09-ccd57b69b993 | Oracle ID: 3ae86510-f828-48f8-adab-759df4b5544e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CETAVOLVER,
    oracle_id = "3ae86510-f828-48f8-adab-759df4b5544e",
    scryfall_id = "69063cc2-4f6e-4cce-bb09-ccd57b69b993",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Red, Color::Blue]),
    faces = &[face!(
        name = "Cetavolver",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::VOLVER],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
