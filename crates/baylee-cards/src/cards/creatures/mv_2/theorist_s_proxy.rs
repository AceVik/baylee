//! Theorist's Proxy — {1}{U} — Creature — Illusion
//! Oracle: Flash
//! Oracle: When this creature enters, empower Jace 3. (Put three loyalty counters on a Jace token you control. If you don't control one, first create a blue Jace planeswalker token with "[−1]: Surveil 1" and "[−3]: Draw a card.")
//! Oracle: {U}, Sacrifice this creature: The next spell you cast this turn can't be countered.
//! Set: FRA #44 — Reality Fracture | Scryfall ID: 710302ca-c4be-4069-8ce1-f531414c74e9 | Oracle ID: 0089acfe-da66-4dd7-b1e5-4d7407f58257

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::THEORIST_S_PROXY,
    oracle_id = "0089acfe-da66-4dd7-b1e5-4d7407f58257",
    scryfall_id = "710302ca-c4be-4069-8ce1-f531414c74e9",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    keywords = KeywordSet::FLASH,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Theorist's Proxy",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ILLUSION],
        power = Some(0),
        toughness = Some(3),
    ),],
    abilities = &[
        triggered!(
            Trigger::ETB,
            &[Effect::EmpowerJace {
                token: &crate::tokens::JACE_PLANESWALKER_BLUE,
                amount: 3,
            }],
        ),
        activated!(
            cost!("{U}", SacrificeSelf),
            &[Effect::continuous(
                &Filter::Any,
                Modifier::NextSpellUncounterable,
                Duration::UntilEndOfTurn,
            )],
        ),
    ],
);
