//! Goblin-town — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {B} or {R}.
//! Oracle: {2}{B}{R}, {T}, Sacrifice this land: Put two +1/+1 counters on target Goblin or Orc you control. Activate only as a sorcery.
//! Set: HOB #183 — The Hobbit | Scryfall ID: d76df9d0-56cf-4351-a5e8-e6ae6fc791d1 | Oracle ID: 98d20908-6d68-4d31-b719-207f93c9b402
// IMPLEMENTED — enters tapped; {T}: Add {B} or {R}; {2}{B}{R}, {T}, Sacrifice this land at sorcery speed to put two +1/+1 counters on a Goblin or Orc you control.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card!(
    index = index::GOBLIN_TOWN,
    oracle_id = "98d20908-6d68-4d31-b719-207f93c9b402",
    scryfall_id = "d76df9d0-56cf-4351-a5e8-e6ae6fc791d1",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Goblin-town",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    abilities = &[
        mana_ability!(&[Effect::mana_choice(&[ManaColor::Black, ManaColor::Red])]),
        activated!(
            cost!("{2}{B}{R}", TapSelf, SacrificeSelf),
            &[Effect::AddCounter {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(2),
            }],
            target = Some(TargetSpec::Object(&Filter::And(&[
                Filter::Or(&[
                    Filter::HasSubtype(creature::GOBLIN),
                    Filter::HasSubtype(creature::ORC),
                ]),
                Filter::ControlledByYou,
            ]))),
            timing = ActivationTiming::SorcerySpeed,
        ),
    ],
);
