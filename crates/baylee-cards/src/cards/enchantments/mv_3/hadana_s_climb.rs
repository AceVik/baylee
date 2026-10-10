//! Hadana's Climb // Winged Temple of Orazca — {1}{G}{U} — Legendary Enchantment // Legendary Land
//! Oracle: At the beginning of combat on your turn, put a +1/+1 counter on target creature you control. Then if that creature has three or more +1/+1 counters on it, transform Hadana's Climb.
//! Oracle: (Transforms from Hadana's Climb.)
//! Oracle: {T}: Add one mana of any color.
//! Oracle: {1}{G}{U}, {T}: Target creature you control gains flying and gets +X/+X until end of turn, where X is its power.
//! Set: RIX #158 — Rivals of Ixalan | Scryfall ID: 8e7554bc-8583-4059-8895-c3845bc27ae3 | Oracle ID: 93b91d18-6acf-42e5-9a31-bc6e01f90c1f
//! Face: Hadana's Climb — {1}{G}{U} — Legendary Enchantment
//! Face: Winged Temple of Orazca —  — Legendary Land

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

static BACK_ABILITIES: &[AbilityDef] = &[
    mana_ability!(&[Effect::mana_of_any_color()]),
    activated!(
        cost!("{1}{G}{U}", TapSelf),
        &[Effect::PumpTarget {
            power: Amount::TargetPower,
            toughness: Amount::TargetPower,
            keywords: KeywordSet::FLYING,
            duration: Duration::UntilEndOfTurn,
        }],
        target = Some(TargetSpec::Object(&Filter::YOUR_CREATURE))
    ),
];

card!(
    index = index::HADANA_S_CLIMB,
    oracle_id = "93b91d18-6acf-42e5-9a31-bc6e01f90c1f",
    scryfall_id = "8e7554bc-8583-4059-8895-c3845bc27ae3",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Blue]),
    faces = &[
        face!(
            name = "Hadana's Climb",
            mana_cost = mana!("{1}{G}{U}"),
            types = TypeSet::ENCHANTMENT,
            supertypes = SupertypeSet::LEGENDARY,
        ),
        face!(
            name = "Winged Temple of Orazca",
            castable_from_hand = false,
            types = TypeSet::LAND,
            supertypes = SupertypeSet::LEGENDARY,
            abilities = BACK_ABILITIES,
        ),
    ],
    coverage = Coverage::Implemented,
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::CombatBegin,
            whose: PlayerRel::You,
        },
        &[
            Effect::AddCounter {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(1),
            },
            Effect::IfTargetMatches {
                filter: &Filter::CountersAtLeast(CounterKind::P1P1, 3),
                then: &[Effect::TransformSource],
            }
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::YOUR_CREATURE)))
    ),],
);
