//! Master of Arms — {2}{W} — Creature — Human Soldier
//! Oracle: First strike
//! Oracle: {1}{W}: Tap target creature blocking this creature.
//! Set: WTH #20 — Weatherlight | Scryfall ID: ac97ff43-c0b6-4f67-ad09-5ba8710c681a | Oracle ID: 37e90cab-d904-4387-9d18-62f125c38c46
// IMPLEMENTED — hand-authored in `baylee-cards`.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static CREATURE_BLOCKING_THIS: Filter = Filter::And(&[Filter::CREATURE, Filter::BlockingSource]);

card!(
    index = index::MASTER_OF_ARMS,
    oracle_id = "37e90cab-d904-4387-9d18-62f125c38c46",
    scryfall_id = "ac97ff43-c0b6-4f67-ad09-5ba8710c681a",
    color_identity = ColorSet::from_slice(&[Color::White]),
    keywords = KeywordSet::FIRST_STRIKE,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Master of Arms",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SOLDIER],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[activated!(
        cost!("{1}{W}"),
        &[Effect::TapTarget],
        target = Some(TargetSpec::Object(&CREATURE_BLOCKING_THIS))
    ),],
);
