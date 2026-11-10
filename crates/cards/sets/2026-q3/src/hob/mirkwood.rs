//! Mirkwood — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {B} or {G}.
//! Oracle: {2}{B}{G}, {T}, Sacrifice this land: Put two +1/+1 counters on target Bear, Spider, or Wolf you control. Activate only as a sorcery.
//! Set: HOB #188 — The Hobbit | Scryfall ID: 612cf954-f86c-4629-99df-4874d56fded3 | Oracle ID: cd49aa99-bf84-4edd-aecc-6dae78b73412
// IMPLEMENTED — enters tapped, {T} for {B} or {G}, and {2}{B}{G}, {T}, sac puts two +1/+1 counters on target Bear, Spider, or Wolf you control at sorcery speed.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

static TARGET_CREATURE: Filter = Filter::And(&[
    Filter::ControlledByYou,
    Filter::Or(&[
        Filter::HasSubtype(creature::BEAR),
        Filter::HasSubtype(creature::SPIDER),
        Filter::HasSubtype(creature::WOLF),
    ]),
]);

card!(
    index = index::MIRKWOOD,
    oracle_id = "cd49aa99-bf84-4edd-aecc-6dae78b73412",
    scryfall_id = "612cf954-f86c-4629-99df-4874d56fded3",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Mirkwood",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    abilities = &[
        mana_ability!(&[Effect::mana_choice(&[ManaColor::Black, ManaColor::Green])]),
        activated!(
            cost!("{2}{B}{G}", TapSelf, SacrificeSelf),
            &[Effect::AddCounter {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(2),
            }],
            target = Some(TargetSpec::Object(&TARGET_CREATURE)),
            timing = ActivationTiming::SorcerySpeed,
        ),
    ],
);
