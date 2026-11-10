//! Gosta Dirk — {3}{W}{W}{U}{U} — Legendary Creature — Human Warrior
//! Oracle: First strike
//! Oracle: Creatures with islandwalk can be blocked as though they didn't have islandwalk.
//! Set: LEG #227 — Legends | Scryfall ID: 92ef316b-dd22-40d1-82e8-8890976684c0 | Oracle ID: fb966147-1d60-4150-b4e7-acf301b2d067
// IMPLEMENTED — first strike, and a layer-6 static strips islandwalk from every creature that has it so it can be blocked as though it didn't.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GOSTA_DIRK,
    oracle_id = "fb966147-1d60-4150-b4e7-acf301b2d067",
    scryfall_id = "92ef316b-dd22-40d1-82e8-8890976684c0",
    color_identity = ColorSet::from_slice(&[Color::Blue, Color::White]),
    commander = CommanderRule::Legendary,
    keywords = KeywordSet::FIRST_STRIKE,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Gosta Dirk",
        mana_cost = mana!("{3}{W}{W}{U}{U}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WARRIOR],
        power = Some(4),
        toughness = Some(4),
    ),],
    abilities = &[static_ability!(
        Filter::And(&[Filter::CREATURE, Filter::HasKeyword(KeywordSet::ISLANDWALK)]),
        Modifier::RemoveKeyword(KeywordSet::ISLANDWALK)
    )],
);
