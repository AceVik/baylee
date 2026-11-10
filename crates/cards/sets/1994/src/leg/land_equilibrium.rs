//! Land Equilibrium — {2}{U}{U} — Enchantment
//! Oracle: If an opponent who controls at least as many lands as you do would put a land onto the battlefield, that player instead puts that land onto the battlefield then sacrifices a land of their choice.
//! Set: ME3 #40 — Masters Edition III | Scryfall ID: 51c5df75-746c-4e4b-84f1-76b689d317d7 | Oracle ID: a3711453-b17d-4b1c-b726-9b41f36d07ab
use baylee_cards_dsl::prelude::*;

card!(
    index = index::LAND_EQUILIBRIUM,
    oracle_id = "a3711453-b17d-4b1c-b726-9b41f36d07ab",
    scryfall_id = "51c5df75-746c-4e4b-84f1-76b689d317d7",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Land Equilibrium",
        mana_cost = mana!("{2}{U}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    // CR 614.13, 616.1: compare before entry; the player putting the land
    // onto the battlefield sacrifices afterward, without intervening priority.
    abilities = &[AbilityDef::Replacement(ReplacementRule::LandEquilibrium)],
);
