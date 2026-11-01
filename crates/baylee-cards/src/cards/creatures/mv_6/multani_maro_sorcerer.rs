//! Multani, Maro-Sorcerer — {4}{G}{G} — Legendary Creature — Elemental Sorcerer
//! Oracle: Shroud (This creature can't be the target of spells or abilities.)
//! Oracle: Multani's power and toughness are each equal to the total number of cards in all players' hands.
//! Set: ULG #107 — Urza's Legacy | Scryfall ID: 0d6cc98b-b376-40af-8308-198bab00b2b1 | Oracle ID: bce5b327-7da1-4136-b76f-069e2fef5d2a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MULTANI_MARO_SORCERER,
    oracle_id = "bce5b327-7da1-4136-b76f-069e2fef5d2a",
    scryfall_id = "0d6cc98b-b376-40af-8308-198bab00b2b1",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Multani, Maro-Sorcerer",
        mana_cost = mana!("{4}{G}{G}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::ELEMENTAL, subtypes::creature::SORCERER],
        power = Some(0),
        toughness = Some(0),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
