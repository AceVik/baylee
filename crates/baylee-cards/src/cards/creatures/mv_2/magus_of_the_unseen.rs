//! Magus of the Unseen — {1}{U} — Creature — Human Wizard
//! Oracle: {1}{U}, {T}: Untap target artifact an opponent controls and gain control of it until end of turn. It gains haste until end of turn. When you lose control of the artifact, tap it.
//! Set: ME2 #53 — Masters Edition II | Scryfall ID: 73c0d236-f283-454f-909f-296dafc4f7da | Oracle ID: d8857977-c8a5-4703-9aaa-331b17c7a713
// NOT SUPPORTED: delayed trigger to tap the artifact when you lose control of it is not supported; untaps target artifact an opponent controls and grants control and haste until end of turn.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static OPPONENT_ARTIFACT: Filter = Filter::And(&[Filter::ARTIFACT, Filter::ControlledByOpponent]);

card!(
    index = index::MAGUS_OF_THE_UNSEEN,
    oracle_id = "d8857977-c8a5-4703-9aaa-331b17c7a713",
    scryfall_id = "73c0d236-f283-454f-909f-296dafc4f7da",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "delayed trigger to tap the artifact when you lose control of it is not supported; untaps target artifact an opponent controls and grants control and haste until end of turn",
    ),
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
        ],
        target = Some(TargetSpec::Object(&OPPONENT_ARTIFACT)),
    )],
);
