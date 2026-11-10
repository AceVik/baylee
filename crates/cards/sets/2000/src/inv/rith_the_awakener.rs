//! Rith, the Awakener — {3}{R}{G}{W} — Legendary Creature — Dragon
//! Oracle: Flying
//! Oracle: Whenever Rith deals combat damage to a player, you may pay {2}{G}. If you do, choose a color, then create a 1/1 green Saproling creature token for each permanent of that color.
//! Set: DMR #198 — Dominaria Remastered | Scryfall ID: e1a75961-39a3-4221-9367-a763cf871d42 | Oracle ID: 97e5d788-2dd3-4c8e-9ab6-f4ea9c7a9a3d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RITH_THE_AWAKENER,
    oracle_id = "97e5d788-2dd3-4c8e-9ab6-f4ea9c7a9a3d",
    scryfall_id = "e1a75961-39a3-4221-9367-a763cf871d42",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Red, Color::White]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Rith, the Awakener",
        mana_cost = mana!("{3}{R}{G}{W}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::DRAGON],
        power = Some(6),
        toughness = Some(6),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
