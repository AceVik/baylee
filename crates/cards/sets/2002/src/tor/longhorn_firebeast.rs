//! Longhorn Firebeast — {2}{R} — Creature — Elemental Ox Beast
//! Oracle: When this creature enters, any opponent may have it deal 5 damage to them. If a player does, sacrifice this creature.
//! Set: TOR #103 — Torment | Scryfall ID: bf0dcf33-8d3f-429c-8ad8-a65d07d7c790 | Oracle ID: c6ca1f21-3aaf-4a5e-938e-7daed67a9d6e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LONGHORN_FIREBEAST,
    oracle_id = "c6ca1f21-3aaf-4a5e-938e-7daed67a9d6e",
    scryfall_id = "bf0dcf33-8d3f-429c-8ad8-a65d07d7c790",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Longhorn Firebeast",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[
            subtypes::creature::ELEMENTAL,
            subtypes::creature::OX,
            subtypes::creature::BEAST
        ],
        power = Some(3),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
