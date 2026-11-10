//! Tethered Skirge — {2}{B} — Creature — Phyrexian Imp
//! Oracle: Flying
//! Oracle: Whenever this creature becomes the target of a spell or ability, you lose 1 life.
//! Set: ULG #70 — Urza's Legacy | Scryfall ID: 0ab1d02c-5d9d-4436-af3b-fb7190c1c028 | Oracle ID: 42bbef66-967b-4922-8b2e-e57d1b8244bb
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TETHERED_SKIRGE,
    oracle_id = "42bbef66-967b-4922-8b2e-e57d1b8244bb",
    scryfall_id = "0ab1d02c-5d9d-4436-af3b-fb7190c1c028",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Tethered Skirge",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::PHYREXIAN, subtypes::creature::IMP],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[triggered!(
        Trigger::BecomesTarget,
        &[Effect::LoseLife {
            amount: Amount::Fixed(1),
            target: PlayerRel::You,
        }],
    )],
);
