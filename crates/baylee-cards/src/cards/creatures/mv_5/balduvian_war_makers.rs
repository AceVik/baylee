//! Balduvian War-Makers — {4}{R} — Creature — Human Barbarian
//! Oracle: Haste
//! Oracle: Rampage 1 (Whenever this creature becomes blocked, it gets +1/+1 until end of turn for each creature blocking it beyond the first.)
//! Set: ALL #66a — Alliances | Scryfall ID: 12fd561e-6a26-4140-a033-1204f5dda5f3 | Oracle ID: 1bda7f60-f986-455b-9d56-bbd494e27c17

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BALDUVIAN_WAR_MAKERS,
    oracle_id = "1bda7f60-f986-455b-9d56-bbd494e27c17",
    scryfall_id = "12fd561e-6a26-4140-a033-1204f5dda5f3",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    keywords = KeywordSet::HASTE,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Balduvian War-Makers",
        mana_cost = mana!("{4}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::BARBARIAN],
        power = Some(3),
        toughness = Some(3),
    ),],
    abilities = &[triggered!(
        Trigger::Rampage,
        &[Effect::PumpFilter {
            filter: &Filter::This,
            controlled_by: None,
            power: Amount::Times {
                base: &Amount::BlockersBeyondFirst,
                factor: 1,
            },
            toughness: Amount::Times {
                base: &Amount::BlockersBeyondFirst,
                factor: 1,
            },
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        }],
    )],
);
