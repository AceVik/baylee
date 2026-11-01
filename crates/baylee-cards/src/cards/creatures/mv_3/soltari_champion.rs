//! Soltari Champion — {2}{W} — Creature — Soltari Soldier
//! Oracle: Shadow (This creature can block or be blocked by only creatures with shadow.)
//! Oracle: Whenever this creature attacks, other creatures you control get +1/+1 until end of turn.
//! Set: TPR #29 — Tempest Remastered | Scryfall ID: b160fb40-daa7-4269-9d9d-93a47ebf9999 | Oracle ID: 765017e5-b43d-4c77-a9a9-0fcf0c0cd098
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static OTHER_CREATURE_YOU_CONTROL: Filter = Filter::And(&[Filter::YOUR_CREATURE, Filter::Another]);

card!(
    index = index::SOLTARI_CHAMPION,
    oracle_id = "765017e5-b43d-4c77-a9a9-0fcf0c0cd098",
    scryfall_id = "b160fb40-daa7-4269-9d9d-93a47ebf9999",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    keywords = KeywordSet::SHADOW,
    faces = &[face!(
        name = "Soltari Champion",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SOLTARI, subtypes::creature::SOLDIER],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[triggered!(
        Trigger::Attacks(&Filter::This),
        &[Effect::PumpFilter {
            filter: &OTHER_CREATURE_YOU_CONTROL,
            controlled_by: None,
            power: Amount::Fixed(1),
            toughness: Amount::Fixed(1),
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        }],
    ),],
);
