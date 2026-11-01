//! Wandering Mage — {W}{U}{B} — Creature — Human Cleric Wizard
//! Oracle: {W}, Pay 1 life: Prevent the next 2 damage that would be dealt to target creature this turn.
//! Oracle: {U}: Prevent the next 1 damage that would be dealt to target Cleric or Wizard creature this turn.
//! Oracle: {B}, Put a -1/-1 counter on a creature you control: Prevent the next 2 damage that would be dealt to target player or planeswalker this turn.
//! Set: ME3 #186 — Masters Edition III | Scryfall ID: 75a5fe4d-ef72-47cd-805d-bfc90755590f | Oracle ID: ac9c81a1-f444-4051-b309-f1af1b5df12a
// IMPLEMENTED — all three damage-prevention activated abilities.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static CLERIC_OR_WIZARD_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Or(&[
        Filter::HasSubtype(subtypes::creature::CLERIC),
        Filter::HasSubtype(subtypes::creature::WIZARD),
    ]),
]);

card!(
    index = index::WANDERING_MAGE,
    oracle_id = "ac9c81a1-f444-4051-b309-f1af1b5df12a",
    scryfall_id = "75a5fe4d-ef72-47cd-805d-bfc90755590f",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue, Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Wandering Mage",
        mana_cost = mana!("{W}{U}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[
            subtypes::creature::HUMAN,
            subtypes::creature::CLERIC,
            subtypes::creature::WIZARD
        ],
        power = Some(0),
        toughness = Some(3),
    ),],
    abilities = &[
        activated!(
            cost!("{W}", PayLife(1)),
            &[Effect::PreventNextDamage {
                target: TargetSpec::Object(&Filter::CREATURE),
                amount: Amount::Fixed(2),
            }],
            target = Some(TargetSpec::Object(&Filter::CREATURE)),
        ),
        activated!(
            cost!("{U}"),
            &[Effect::PreventNextDamage {
                target: TargetSpec::Object(&CLERIC_OR_WIZARD_CREATURE),
                amount: Amount::Fixed(1),
            }],
            target = Some(TargetSpec::Object(&CLERIC_OR_WIZARD_CREATURE)),
        ),
        activated!(
            cost!(
                "{B}",
                PutCounterOnOther {
                    filter: &Filter::YOUR_CREATURE,
                    kind: CounterKind::M1M1,
                    n: 1,
                }
            ),
            &[Effect::PreventNextDamage {
                target: TargetSpec::PlayerOrObject(&Filter::PLANESWALKER),
                amount: Amount::Fixed(2),
            }],
            target = Some(TargetSpec::PlayerOrObject(&Filter::PLANESWALKER)),
        ),
    ],
);
