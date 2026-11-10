//! Ojer Taq, Deepest Foundation // Temple of Civilization — {4}{W}{W} — Legendary Creature — God // Land
//! Oracle: Vigilance
//! Oracle: If one or more creature tokens would be created under your control, three times that many of those tokens are created instead.
//! Oracle: When Ojer Taq dies, return it to the battlefield tapped and transformed under its owner's control.
//! Oracle: (Transforms from Ojer Taq, Deepest Foundation.)
//! Oracle: {T}: Add {W}.
//! Oracle: {2}{W}, {T}: Transform this land. Activate only if you attacked with three or more creatures this turn and only as a sorcery.
//! Set: LCI #26 — The Lost Caverns of Ixalan | Scryfall ID: 1ca79dd4-67fc-496c-96fc-489b039c4932 | Oracle ID: 486bb9a5-73f1-4cec-b097-fb07ac80b72e
//! Face: Ojer Taq, Deepest Foundation — {4}{W}{W} — Legendary Creature — God
//! Face: Temple of Civilization —  — Land

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static TEMPLE_MANA: &[AbilityDef] = &[
    mana_ability!(&[Effect::mana(ManaColor::White, 1)]),
    activated!(
        cost!("{2}{W}", TapSelf),
        &[Effect::TransformSource],
        timing = ActivationTiming::SorcerySpeed,
        condition = Some(Condition::AttackedWithAtLeast(3))
    ),
];

card!(
    index = index::OJER_TAQ_DEEPEST_FOUNDATION,
    oracle_id = "486bb9a5-73f1-4cec-b097-fb07ac80b72e",
    scryfall_id = "1ca79dd4-67fc-496c-96fc-489b039c4932",
    color_identity = ColorSet::from_slice(&[Color::White]),
    commander = CommanderRule::Legendary,
    faces = &[
        face!(
            name = "Ojer Taq, Deepest Foundation",
            mana_cost = mana!("{4}{W}{W}"),
            types = TypeSet::CREATURE,
            supertypes = SupertypeSet::LEGENDARY,
            subtypes = &[subtypes::creature::GOD],
            power = Some(6),
            toughness = Some(6),
            keywords = KeywordSet::VIGILANCE,
        ),
        face!(
            name = "Temple of Civilization",
            castable_from_hand = false,
            types = TypeSet::LAND,
            abilities = TEMPLE_MANA,
        ),
    ],
    coverage = Coverage::Implemented,
    abilities = &[
        AbilityDef::Replacement(ReplacementRule::MultiplyTypedTokens {
            controller_filter: &Filter::ControlledByYou,
            types: TypeSet::CREATURE,
            factor: 3
        }),
        triggered!(
            Trigger::Dies(&Filter::This),
            &[Effect::ReturnSourceFromGraveyard {
                face: Some(1),
                tapped: true,
                owner_control: true,
                counters: None,
            }]
        ),
    ],
);
