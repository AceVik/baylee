//! Gorilla Berserkers — {3}{G}{G} — Creature — Ape Berserker
//! Oracle: Trample; rampage 2 (Whenever this creature becomes blocked, it gets +2/+2 until end of turn for each creature blocking it beyond the first.)
//! Oracle: This creature can't be blocked except by three or more creatures.
//! Set: ALL #93a — Alliances | Scryfall ID: 344b4613-17f8-4c8b-b5bc-f773a8f8007a | Oracle ID: 6373c66d-edfc-44cb-af2a-2273e0ba53e0
// IMPLEMENTED: trample; rampage 2; can't be blocked except by three or more creatures.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GORILLA_BERSERKERS,
    oracle_id = "6373c66d-edfc-44cb-af2a-2273e0ba53e0",
    scryfall_id = "344b4613-17f8-4c8b-b5bc-f773a8f8007a",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Gorilla Berserkers",
        mana_cost = mana!("{3}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::APE, subtypes::creature::BERSERKER],
        power = Some(2),
        toughness = Some(3),
        keywords = KeywordSet::TRAMPLE,
    ),],
    abilities = &[
        static_ability!(Filter::This, Modifier::MinimumBlockers(3)),
        triggered!(
            Trigger::Rampage,
            &[Effect::PumpFilter {
                filter: &Filter::This,
                controlled_by: None,
                power: Amount::Times {
                    base: &Amount::BlockersBeyondFirst,
                    factor: 2,
                },
                toughness: Amount::Times {
                    base: &Amount::BlockersBeyondFirst,
                    factor: 2,
                },
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn,
            }],
        )
    ],
);
