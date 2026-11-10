//! Shaleskin Bruiser — {6}{R} — Creature — Beast
//! Oracle: Trample
//! Oracle: Whenever this creature attacks, it gets +3/+0 until end of turn for each other attacking Beast.
//! Set: ONS #226 — Onslaught | Scryfall ID: fc2de8a4-0d84-4f7c-bbe4-3a31172186ab | Oracle ID: b90e370a-5080-485e-a957-93d5f60e6cdb

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SHALESKIN_BRUISER,
    oracle_id = "b90e370a-5080-485e-a957-93d5f60e6cdb",
    scryfall_id = "fc2de8a4-0d84-4f7c-bbe4-3a31172186ab",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    keywords = KeywordSet::TRAMPLE,
    coverage = Coverage::Implemented,
    abilities = &[triggered!(
        Trigger::Attacks(&Filter::This),
        &[Effect::PumpFilter {
            filter: &Filter::This,
            controlled_by: None,
            power: Amount::Times {
                base: &Amount::CountOf {
                    filter: &Filter::And(&[
                        Filter::Another,
                        Filter::Attacking,
                        Filter::HasSubtype(subtypes::creature::BEAST)
                    ]),
                    zone: ZoneSel::Battlefield,
                },
                factor: 3
            },
            toughness: Amount::Fixed(0),
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        }]
    )],
    faces = &[face!(
        name = "Shaleskin Bruiser",
        mana_cost = mana!("{6}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BEAST],
        power = Some(4),
        toughness = Some(4),
    ),],
);
