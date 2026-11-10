//! Craw Giant — {3}{G}{G}{G}{G} — Creature — Giant
//! Oracle: Trample
//! Oracle: Rampage 2 (Whenever this creature becomes blocked, it gets +2/+2 until end of turn for each creature blocking it beyond the first.)
//! Set: TSB #76 — Time Spiral Timeshifted | Scryfall ID: 81c5ee8f-a060-486c-a7f1-7bd600f6911a | Oracle ID: 152b621e-4c18-438c-9999-77e2685fda79

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CRAW_GIANT,
    oracle_id = "152b621e-4c18-438c-9999-77e2685fda79",
    scryfall_id = "81c5ee8f-a060-486c-a7f1-7bd600f6911a",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    keywords = KeywordSet::TRAMPLE,
    coverage = Coverage::Implemented,
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
    )],
    faces = &[face!(
        name = "Craw Giant",
        mana_cost = mana!("{3}{G}{G}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GIANT],
        power = Some(6),
        toughness = Some(4),
    ),],
);
