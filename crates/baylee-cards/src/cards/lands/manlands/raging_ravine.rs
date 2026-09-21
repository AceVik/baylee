//! Raging Ravine — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {R} or {G}.
//! Oracle: {2}{R}{G}: Until end of turn, this land becomes a 3/3 red and green Elemental creature with "Whenever this creature attacks, put a +1/+1 counter on it." It's still a land.
//! Set: ECC #160 — Lorwyn Eclipsed Commander | Scryfall ID: 3964c3a5-3ce7-4b73-89ec-ed1b9799aef8 | Oracle ID: 8d38194e-b607-4ff4-9c19-0e8636d463bf
// IMPLEMENTED — enters tapped; {T}: Add {R} or {G}; {2}{R}{G} turns it into a 3/3 red and green Elemental until end of turn (still a land) and hands it the attack trigger that counters itself up.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card!(
    index = index::RAGING_RAVINE,
    oracle_id = "8d38194e-b607-4ff4-9c19-0e8636d463bf",
    scryfall_id = "3964c3a5-3ce7-4b73-89ec-ed1b9799aef8",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Red]),
    faces = &[face!(
        name = "Raging Ravine",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana_choice(&[ManaColor::Red, ManaColor::Green])]),
        activated!(
            cost!("{2}{R}{G}"),
            &[
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddType(TypeSet::CREATURE),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddColor(ColorSet::from_slice(&[Color::Red, Color::Green])),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddSubtype(creature::ELEMENTAL),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::SetPT(3, 3),
                    Duration::UntilEndOfTurn
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::GrantTriggered {
                        trigger: Trigger::Attacks(&Filter::This),
                        effects: &[Effect::AddCounter {
                            kind: CounterKind::P1P1,
                            amount: Amount::Fixed(1),
                        }],
                        target: None,
                    },
                    Duration::UntilEndOfTurn,
                ),
            ]
        ),
    ],
);
