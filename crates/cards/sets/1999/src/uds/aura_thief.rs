//! Aura Thief — {3}{U} — Creature — Illusion
//! Oracle: Flying
//! Oracle: When this creature dies, you gain control of all enchantments. (You don't get to move Auras.)
//! Set: UDS #26 — Urza's Destiny | Scryfall ID: 8ae591d2-b9d3-4bc5-bcec-5d3d79a13b41 | Oracle ID: cf0936ea-2741-43a4-ad6a-a1258b54945b
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::AURA_THIEF,
    oracle_id = "cf0936ea-2741-43a4-ad6a-a1258b54945b",
    scryfall_id = "8ae591d2-b9d3-4bc5-bcec-5d3d79a13b41",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Aura Thief",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ILLUSION],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[triggered!(
        Trigger::Dies(&Filter::This),
        &[Effect::GainControlAll {
            filter: &Filter::ENCHANTMENT,
            to: PlayerRel::You,
        }],
    )],
);
