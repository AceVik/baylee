//! Mistform Ultimus — {3}{U} — Legendary Creature — Illusion
//! Oracle: Mistform Ultimus is every creature type (even if this card isn't on the battlefield).
//! Set: TSB #26 — Time Spiral Timeshifted | Scryfall ID: b74feed4-840b-4f10-9861-7a6885ce7907 | Oracle ID: 7bf0e628-06b5-4e12-a526-a2bce08844f2
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MISTFORM_ULTIMUS,
    oracle_id = "7bf0e628-06b5-4e12-a526-a2bce08844f2",
    scryfall_id = "b74feed4-840b-4f10-9861-7a6885ce7907",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Mistform Ultimus",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::ILLUSION],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
