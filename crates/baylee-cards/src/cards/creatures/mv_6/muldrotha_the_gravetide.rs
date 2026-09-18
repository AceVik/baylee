//! Muldrotha, the Gravetide — {3}{B}{G}{U} — Legendary Creature — Elemental Avatar
//! Oracle: During each of your turns, you may play a land and cast a permanent spell of each permanent type from your graveyard. (If a card has multiple permanent types, choose one as you play it.)
//! Set: ECC #128 — Lorwyn Eclipsed Commander | Scryfall ID: 705b4d97-2f50-47f7-9053-d748f4337553 | Oracle ID: e4625704-1d52-44e4-804f-2f45644d76ac
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MULDROTHA_THE_GRAVETIDE,
    oracle_id = "e4625704-1d52-44e4-804f-2f45644d76ac",
    scryfall_id = "705b4d97-2f50-47f7-9053-d748f4337553",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green, Color::Blue]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Muldrotha, the Gravetide",
        mana_cost = mana!("{3}{B}{G}{U}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::ELEMENTAL, subtypes::creature::AVATAR],
        power = Some(6),
        toughness = Some(6),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
