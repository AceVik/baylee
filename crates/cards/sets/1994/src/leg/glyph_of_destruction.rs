//! Glyph of Destruction — {R} — Instant
//! Oracle: Target blocking Wall you control gets +10/+0 until end of combat. Prevent all damage that would be dealt to it this turn. Destroy it at the beginning of the next end step.
//! Set: LEG #150 — Legends | Scryfall ID: 8e9c153c-9224-491b-bc84-8a9f0a83ee5a | Oracle ID: cca68900-2093-4cae-b757-efc7dd807e18

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GLYPH_OF_DESTRUCTION,
    oracle_id = "cca68900-2093-4cae-b757-efc7dd807e18",
    scryfall_id = "8e9c153c-9224-491b-bc84-8a9f0a83ee5a",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Glyph of Destruction",
        mana_cost = mana!("{R}"),
        types = TypeSet::INSTANT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[
            Effect::PumpTarget {
                power: Amount::Fixed(10),
                toughness: Amount::Fixed(0),
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfCombat,
            },
            Effect::continuous(
                &Filter::This,
                Modifier::PreventDamageFrom(&Filter::Any),
                Duration::UntilEndOfTurn
            ),
            Effect::AtNextEndStep {
                effects: &[Effect::destroy(TargetSpec::EventObject)]
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::And(&[
            Filter::HasSubtype(subtypes::creature::WALL),
            Filter::Blocking,
            Filter::ControlledByYou,
        ]))))
    )],
);
