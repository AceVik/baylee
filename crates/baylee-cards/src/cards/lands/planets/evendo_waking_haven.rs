//! Evendo, Waking Haven — (no cost) — Land — Planet
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {G}.
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Planet. Station only as a sorcery.)
//! Oracle: 12+ | {G}, {T}: Add {G} for each creature you control.
//! Set: EOE #253 — Edge of Eternities | Scryfall ID: 2fa09104-acbe-4410-b101-2fe6ac28efde | Oracle ID: 83161d59-2520-4741-9328-e2a4a8b5d5bc
// PARTIAL — enters tapped, {T}: Add {G}, and the 12+ charge-counter mana
// line; Station is off the card, see NOT SUPPORTED below.

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
    coverage = Coverage::Partial(
        "Station — put charge counters equal to the power of the creature tapped as its cost: no Amount reads an object named by a cost",
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Green, 1)]),
        mana_ability!(
            cost!("{G}", TapSelf),
            &[Effect::mana_dynamic(
                ManaColor::Green,
                Amount::CountOf {
                    filter: &Filter::YOUR_CREATURE,
                    zone: ZoneSel::Battlefield,
                },
            )],
            condition = Some(Condition::CountersOnSelf(CounterKind::Charge, 12)),
        ),
    ],
);

// NOT SUPPORTED: Station — "Put charge counters equal to its power on this
// Planet. Station only as a sorcery." The cost half is expressible
// (`CostPart::TapOther` over another creature you control, at
// `ActivationTiming::SorcerySpeed`), but the amount is not: no `Amount`
// variant reaches an object named by a cost, and `Amount::TargetPower`
// would mean the ability *targets* the creature — which the printed
// sentence does not say and which hexproof would then answer. An ability
// cannot be written at a wrong size, so it comes off the card.
//
// The 12+ line is written as printed: `Condition::CountersOnSelf(Charge, 12)`
// reads the charge counters Station would have put there. Those counters
// cannot come from this card any more, so the gate only opens under another
// card's counters.
