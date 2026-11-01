//! Resolute Archangel — {5}{W}{W} — Creature — Angel
//! Oracle: Flying
//! Oracle: When this creature enters, if your life total is less than your starting life total, it becomes equal to your starting life total.
//! Set: M15 #28 — Magic 2015 | Scryfall ID: e37b31b8-d868-4dff-9ab0-723ce41ee7e4 | Oracle ID: 0d6dc95d-55ce-4ef8-b065-d7203f5bf642
// IMPLEMENTED — flying and conditional restoration to starting life.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RESOLUTE_ARCHANGEL,
    oracle_id = "0d6dc95d-55ce-4ef8-b065-d7203f5bf642",
    scryfall_id = "e37b31b8-d868-4dff-9ab0-723ce41ee7e4",
    color_identity = ColorSet::from_slice(&[Color::White]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Resolute Archangel",
        mana_cost = mana!("{5}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ANGEL],
        power = Some(4),
        toughness = Some(4),
    ),],
    abilities = &[triggered!(
        Trigger::ETB,
        &[Effect::RaiseLifeTotalTo {
            who: PlayerRel::You,
            amount: Amount::StartingLifeTotal(PlayerRel::You),
        }],
        condition = Some(Condition::LifeBelowStarting)
    )],
);
