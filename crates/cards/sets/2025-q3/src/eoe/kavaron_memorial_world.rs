//! Kavaron, Memorial World — (no cost) — Land — Planet
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {R}.
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Planet. Station only as a sorcery.)
//! Oracle: 12+ | {1}{R}, {T}, Sacrifice a land: Create a 2/2 colorless Robot artifact creature token, then creatures you control get +1/+0 and gain haste until end of turn.
//! Set: EOE #255 — Edge of Eternities | Scryfall ID: 60f3ca25-9dcc-4781-bf7b-ab6736d8db29 | Oracle ID: 4fa826ca-d361-4391-ad0d-989ebcfa4a91

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::KAVARON_MEMORIAL_WORLD,
    oracle_id = "4fa826ca-d361-4391-ad0d-989ebcfa4a91",
    scryfall_id = "60f3ca25-9dcc-4781-bf7b-ab6736d8db29",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Kavaron, Memorial World",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::PLANET],
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Red, 1)]),
        activated!(
            cost!("{1}{R}", TapSelf, Sacrifice(&Filter::LAND)),
            &[
                Effect::CreateToken {
                    token: &generated_tokens::ROBOT_ARTIFACT_2_2,
                },
                Effect::PumpFilter {
                    filter: &Filter::YOUR_CREATURE,
                    controlled_by: None,
                    power: Amount::Fixed(1),
                    toughness: Amount::Fixed(0),
                    keywords: KeywordSet::HASTE,
                    duration: Duration::UntilEndOfTurn,
                },
            ],
            condition = Some(Condition::CountersOnSelf(CounterKind::Charge, 12)),
        ),
        activated!(
            cost!(TapOther(&Filter::ANOTHER_CREATURE_YOU_CONTROL)),
            &[Effect::AddCounter {
                kind: CounterKind::Charge,
                amount: Amount::TappedPower
            }],
            timing = ActivationTiming::SorcerySpeed,
        ),
    ],
);
