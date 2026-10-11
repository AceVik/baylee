//! Llanowar Reborn — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {G}.
//! Oracle: Graft 1 (This land enters with a +1/+1 counter on it. Whenever a creature enters, you may move a +1/+1 counter from this land onto it.)
//! Set: LCC #341 — The Lost Caverns of Ixalan Commander | Scryfall ID: 284e165e-9ab6-47fc-9685-0fb03de237df | Oracle ID: 92acb789-0e42-465c-ac16-40fefec48805

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LLANOWAR_REBORN,
    oracle_id = "92acb789-0e42-465c-ac16-40fefec48805",
    scryfall_id = "284e165e-9ab6-47fc-9685-0fb03de237df",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Llanowar Reborn",
        types = TypeSet::LAND,
        enter_modifiers = &[
            EnterModifier::Tapped,
            EnterModifier::WithCounters {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(1),
            },
        ],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Green, 1)]),
        triggered!(
            Trigger::EntersBattlefield(&Filter::And(&[Filter::CREATURE, Filter::Another])),
            &[Effect::MayDo {
                effects: &[Effect::MoveCounterToEvent {
                    kind: CounterKind::P1P1
                }]
            }],
            condition = Some(Condition::CountersOnSelf(CounterKind::P1P1, 1))
        ),
    ],
);
