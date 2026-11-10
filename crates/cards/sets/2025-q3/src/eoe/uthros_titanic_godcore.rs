//! Uthros, Titanic Godcore — (no cost) — Land — Planet
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {U}.
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Planet. Station only as a sorcery.)
//! Oracle: 12+ | {U}, {T}: Add {U} for each artifact you control.
//! Set: EOE #260 — Edge of Eternities | Scryfall ID: 11da39d6-cfa6-498d-91b1-11454cc7e5a3 | Oracle ID: df08ac72-010f-42f8-beb3-6d645c638e1e

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::UTHROS_TITANIC_GODCORE,
    oracle_id = "df08ac72-010f-42f8-beb3-6d645c638e1e",
    scryfall_id = "11da39d6-cfa6-498d-91b1-11454cc7e5a3",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Uthros, Titanic Godcore",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::PLANET],
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Blue, 1)]),
        mana_ability!(
            cost!("{U}", TapSelf),
            &[Effect::mana_dynamic(
                ManaColor::Blue,
                Amount::CountOf {
                    filter: &Filter::YOUR_ARTIFACT,
                    zone: ZoneSel::Battlefield,
                },
            )],
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
