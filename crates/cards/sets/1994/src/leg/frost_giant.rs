//! Frost Giant — {3}{R}{R}{R} — Creature — Giant
//! Oracle: Rampage 2 (Whenever this creature becomes blocked, it gets +2/+2 until end of turn for each creature blocking it beyond the first.)
//! Set: ME3 #101 — Masters Edition III | Scryfall ID: a742bfa4-3730-4bfd-865c-92fb4b40f2f2 | Oracle ID: 56ed6af3-8f85-4907-9364-eca71f21bb11

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FROST_GIANT,
    oracle_id = "56ed6af3-8f85-4907-9364-eca71f21bb11",
    scryfall_id = "a742bfa4-3730-4bfd-865c-92fb4b40f2f2",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Frost Giant",
        mana_cost = mana!("{3}{R}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GIANT],
        power = Some(4),
        toughness = Some(4),
    ),],
    abilities = &[triggered!(
        Trigger::Rampage,
        &[Effect::PumpFilter {
            filter: &Filter::This,
            controlled_by: None,
            power: Amount::Times {
                base: &Amount::BlockersBeyondFirst,
                factor: 2
            },
            toughness: Amount::Times {
                base: &Amount::BlockersBeyondFirst,
                factor: 2
            },
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        }]
    ),],
);
