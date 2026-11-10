//! Autumn Willow — {4}{G}{G} — Legendary Creature — Avatar
//! Oracle: Shroud (This creature can't be the target of spells or abilities.)
//! Oracle: {G}: Until end of turn, Autumn Willow can be the target of spells and abilities controlled by target player as though it didn't have shroud.
//! Set: ME1 #113 — Masters Edition | Scryfall ID: 9d0a2c3b-ab86-44af-919e-c95c8751157e | Oracle ID: c8c868f8-19a5-4cc5-b769-aa382f7b97ad
// PARTIAL — per-player shroud bypass until end of turn is not in Effect/Modifier; has Shroud and {G} target-player activated ability.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::AUTUMN_WILLOW,
    oracle_id = "c8c868f8-19a5-4cc5-b769-aa382f7b97ad",
    scryfall_id = "9d0a2c3b-ab86-44af-919e-c95c8751157e",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    commander = CommanderRule::Legendary,
    keywords = KeywordSet::SHROUD,
    coverage = Coverage::Partial(
        "per-player shroud bypass until end of turn not in Effect/Modifier; has Shroud and {G} target-player activated ability"
    ),
    faces = &[face!(
        name = "Autumn Willow",
        mana_cost = mana!("{4}{G}{G}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::AVATAR],
        power = Some(4),
        toughness = Some(4),
    ),],
    abilities = &[activated!(
        cost!("{G}"),
        &[],
        targets = Some(TargetReq::one(TargetSpec::AnyPlayer))
    )],
);
