//! Magus of the Unseen — {1}{U} — Creature — Human Wizard
//! Oracle: {1}{U}, {T}: Untap target artifact an opponent controls and gain control of it until end of turn. It gains haste until end of turn. When you lose control of the artifact, tap it.
//! Set: ME2 #53 — Masters Edition II | Scryfall ID: 73c0d236-f283-454f-909f-296dafc4f7da | Oracle ID: d8857977-c8a5-4703-9aaa-331b17c7a713

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MAGUS_OF_THE_UNSEEN,
    oracle_id = "d8857977-c8a5-4703-9aaa-331b17c7a713",
    scryfall_id = "73c0d236-f283-454f-909f-296dafc4f7da",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Magus of the Unseen",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!("{1}{U}", TapSelf),
        &[
            Effect::UntapTarget,
            Effect::continuous(
                &Filter::This,
                Modifier::GainControl,
                Duration::UntilEndOfTurn,
            ),
            Effect::PumpTarget {
                power: Amount::Fixed(0),
                toughness: Amount::Fixed(0),
                keywords: KeywordSet::HASTE,
                duration: Duration::UntilEndOfTurn,
            },
            Effect::CreateDelayedObserver {
                watch: TargetSpec::AnyTarget,
                when: DelayedObservation::YouLoseControl,
                about: TargetSpec::AnyTarget,
                effects: &[Effect::TapEventObject],
            },
        ],
        target = Some(TargetSpec::Object(&Filter::And(&[
            Filter::ARTIFACT,
            Filter::ControlledByOpponent,
        ]))),
    )],
);
