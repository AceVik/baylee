//! Krovikan Elementalist — {B}{B} — Creature — Human Wizard
//! Oracle: {2}{R}: Target creature gets +1/+0 until end of turn.
//! Oracle: {U}{U}: Target creature you control gains flying until end of turn. Sacrifice it at the beginning of the next end step.
//! Set: ICE #139 — Ice Age | Scryfall ID: bbedca18-a074-4441-b0a9-7b14fdb07412 | Oracle ID: 43e6fc44-8b23-4bff-91da-936019efe84b

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::KROVIKAN_ELEMENTALIST,
    oracle_id = "43e6fc44-8b23-4bff-91da-936019efe84b",
    scryfall_id = "bbedca18-a074-4441-b0a9-7b14fdb07412",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red, Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Krovikan Elementalist",
        mana_cost = mana!("{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[
        activated!(
            cost!("{2}{R}"),
            &[Effect::PumpTarget {
                power: Amount::Fixed(1),
                toughness: Amount::Fixed(0),
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn,
            }],
            target = Some(TargetSpec::Object(&Filter::CREATURE)),
        ),
        activated!(
            cost!("{U}{U}"),
            &[
                Effect::PumpTarget {
                    power: Amount::Fixed(0),
                    toughness: Amount::Fixed(0),
                    keywords: KeywordSet::FLYING,
                    duration: Duration::UntilEndOfTurn,
                },
                Effect::AtNextEndStep {
                    effects: &[Effect::SacrificeEvent],
                },
            ],
            target = Some(TargetSpec::Object(&Filter::YOUR_CREATURE)),
        ),
    ],
);
