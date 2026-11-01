//! Shivan Phoenix — {4}{R}{R} — Creature — Phoenix
//! Oracle: Flying
//! Oracle: When this creature dies, return it to its owner's hand.
//! Set: ULG #91 — Urza's Legacy | Scryfall ID: 112aa0e2-7e4a-4ae8-bedb-d84b4116df5e | Oracle ID: 49ca8516-624b-4aa8-8e13-d145590cdfda
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SHIVAN_PHOENIX,
    oracle_id = "49ca8516-624b-4aa8-8e13-d145590cdfda",
    scryfall_id = "112aa0e2-7e4a-4ae8-bedb-d84b4116df5e",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Shivan Phoenix",
        mana_cost = mana!("{4}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::PHOENIX],
        power = Some(3),
        toughness = Some(4),
    ),],
    abilities = &[triggered!(
        Trigger::Dies(&Filter::This),
        &[Effect::ReturnToHand {
            target: TargetSpec::ThisObject,
        }],
    )],
);
