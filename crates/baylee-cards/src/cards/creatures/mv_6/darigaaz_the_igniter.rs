//! Darigaaz, the Igniter — {3}{B}{R}{G} — Legendary Creature — Dragon
//! Oracle: Flying
//! Oracle: Whenever Darigaaz deals combat damage to a player, you may pay {2}{R}. If you do, choose a color, then that player reveals their hand and Darigaaz deals damage to the player equal to the number of cards of that color revealed this way.
//! Set: DDE #47 — Duel Decks: Phyrexia vs. the Coalition | Scryfall ID: 942acc59-3644-47be-80ab-fc8f27626aa3 | Oracle ID: 6cbafa7b-4448-4ad4-b4f2-0ab6e7c95b0b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DARIGAAZ_THE_IGNITER,
    oracle_id = "6cbafa7b-4448-4ad4-b4f2-0ab6e7c95b0b",
    scryfall_id = "942acc59-3644-47be-80ab-fc8f27626aa3",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green, Color::Red]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Darigaaz, the Igniter",
        mana_cost = mana!("{3}{B}{R}{G}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::DRAGON],
        power = Some(6),
        toughness = Some(6),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
