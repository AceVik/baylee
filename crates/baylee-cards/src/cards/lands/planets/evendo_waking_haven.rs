//! Evendo, Waking Haven — (no cost) — Land — Planet
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {G}.
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Planet. Station only as a sorcery.)
//! Oracle: 12+ | {G}, {T}: Add {G} for each creature you control.
//! Set: EOE #253 — Edge of Eternities | Scryfall ID: 2fa09104-acbe-4410-b101-2fe6ac28efde | Oracle ID: 83161d59-2520-4741-9328-e2a4a8b5d5bc
// IMPLEMENTED — enters tapped, {T}: Add {G}, station (tap another creature
// you control as the cost, its power in charge counters, sorcery speed), and
// the 12+ mana ability.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::EVENDO_WAKING_HAVEN,
    oracle_id = "83161d59-2520-4741-9328-e2a4a8b5d5bc",
    scryfall_id = "2fa09104-acbe-4410-b101-2fe6ac28efde",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Evendo, Waking Haven",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::PLANET],
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Green, 1)]),
        // Station (CR 702.184a): "Tap another untapped creature you
        // control: Put a number of charge counters on this permanent equal
        // to the tapped creature's power. Activate only as a sorcery." The
        // creature is a cost, not a target.
        activated!(
            cost!(TapOther(&Filter::ANOTHER_CREATURE_YOU_CONTROL)),
            &[Effect::AddCounter {
                kind: CounterKind::Charge,
                amount: Amount::TappedPower,
            }],
            timing = ActivationTiming::SorcerySpeed
        ),
        // 12+ (CR 721.2a): it has this ability while it has twelve or more
        // charge counters.
        mana_ability!(
            cost!("{G}", TapSelf),
            &[Effect::mana_dynamic(
                ManaColor::Green,
                Amount::CountOf {
                    filter: &Filter::YOUR_CREATURE,
                    zone: ZoneSel::Battlefield,
                },
            )],
            condition = Some(Condition::Station(12)),
        ),
    ],
);
