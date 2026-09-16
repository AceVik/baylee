//! Derevi, Empyrial Tactician — {G}{W}{U} — Legendary Creature — Bird Wizard
//! Oracle: Flying
//! Oracle: When Derevi enters and whenever a creature you control deals combat damage to a player, you may tap or untap target permanent.
//! Oracle: {1}{G}{W}{U}: Put Derevi onto the battlefield from the command zone.
//! Set: CMA #176 — Commander Anthology | Scryfall ID: 3a1d0dad-18a8-489e-ac11-08f64b72fda4 | Oracle ID: afa49a09-146f-4439-850e-dd1938c93cef
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DEREVI_EMPYRIAL_TACTICIAN,
    oracle_id = "afa49a09-146f-4439-850e-dd1938c93cef",
    scryfall_id = "3a1d0dad-18a8-489e-ac11-08f64b72fda4",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Blue, Color::White]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Derevi, Empyrial Tactician",
        mana_cost = mana!("{G}{W}{U}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::BIRD, subtypes::creature::WIZARD],
        power = Some(2),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
