//! The Balrog of Moria — {4}{B}{B}{R} — Legendary Creature — Avatar Demon
//! Oracle: Trample, haste
//! Oracle: When The Balrog of Moria dies, you may exile it. When you do, for each opponent, exile up to one target creature that player controls.
//! Oracle: Cycling {3}{R} ({3}{R}, Discard this card: Draw a card.)
//! Oracle: When you cycle this card, create two Treasure tokens.
//! Set: LTC #46 — Tales of Middle-earth Commander | Scryfall ID: 2a3fcfdc-f2cf-42d8-8bb4-7308bc12746e | Oracle ID: d73191d8-6f94-4fba-acd2-2d0490e3ac00
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::THE_BALROG_OF_MORIA,
    oracle_id = "d73191d8-6f94-4fba-acd2-2d0490e3ac00",
    scryfall_id = "2a3fcfdc-f2cf-42d8-8bb4-7308bc12746e",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "The Balrog of Moria",
        mana_cost = mana!("{4}{B}{B}{R}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::AVATAR, subtypes::creature::DEMON],
        power = Some(8),
        toughness = Some(8),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
