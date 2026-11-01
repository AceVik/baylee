//! Soltari Visionary — {1}{W}{W} — Creature — Soltari Cleric
//! Oracle: Shadow (This creature can block or be blocked by only creatures with shadow.)
//! Oracle: Whenever this creature deals damage to a player, destroy target enchantment that player controls.
//! Set: EXO #20 — Exodus | Scryfall ID: 1a3ae384-7b60-4264-9dc1-1613917168ca | Oracle ID: 7971f5c2-97b7-441f-ad77-954f43ede0a1
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SOLTARI_VISIONARY,
    oracle_id = "7971f5c2-97b7-441f-ad77-954f43ede0a1",
    scryfall_id = "1a3ae384-7b60-4264-9dc1-1613917168ca",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Soltari Visionary",
        mana_cost = mana!("{1}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SOLTARI, subtypes::creature::CLERIC],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
