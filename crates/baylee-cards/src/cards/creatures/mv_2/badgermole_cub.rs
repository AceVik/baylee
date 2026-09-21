//! Badgermole Cub — {1}{G} — Creature — Badger Mole
//! Oracle: When this creature enters, earthbend 1. (Target land you control becomes a 0/0 creature with haste that's still a land. Put a +1/+1 counter on it. When it dies or is exiled, return it to the battlefield tapped.)
//! Oracle: Whenever you tap a creature for mana, add an additional {G}.
//! Set: TLA #167 — Avatar: The Last Airbender | Scryfall ID: 340c5799-4964-44dd-8c48-8f3f3aba5211 | Oracle ID: 2b0afb89-0944-4861-b9c3-e909e2ac215e
// PARTIAL — earthbend's animation is built out of three continuous effects
// (0/0 creature, haste, still a land) plus its +1/+1 counter; the keyword's
// return rider and the tap-for-mana clause have no DSL shape.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BADGERMOLE_CUB,
    oracle_id = "2b0afb89-0944-4861-b9c3-e909e2ac215e",
    scryfall_id = "340c5799-4964-44dd-8c48-8f3f3aba5211",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Badgermole Cub",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BADGER, subtypes::creature::MOLE],
        power = Some(2),
        toughness = Some(2),
    ),],
    coverage = Coverage::Partial(
        "earthbend's \"when it dies or is exiled, return it to the battlefield \
         tapped\" rider and \"whenever you tap a creature for mana, add an \
         additional {G}\" have no DSL variant",
    ),
    abilities = &[
        // When this creature enters, earthbend 1: target land you control
        // becomes a 0/0 creature with haste that's still a land (the filter
        // is `This`, which in a created effect is the ability's target), and
        // a +1/+1 counter goes on the same land.
        triggered!(
            Trigger::ETB,
            &[
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddType(TypeSet::CREATURE),
                    Duration::Indefinitely,
                ),
                Effect::continuous(&Filter::This, Modifier::SetPT(0, 0), Duration::Indefinitely,),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddKeyword(KeywordSet::HASTE),
                    Duration::Indefinitely,
                ),
                Effect::AddCounter {
                    kind: CounterKind::P1P1,
                    amount: Amount::Fixed(1),
                },
            ],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::YOUR_LAND)))
        ),
        // NOT SUPPORTED: "When it dies or is exiled, return it to the
        // battlefield tapped." — earthbend registers a delayed trigger that
        // watches the permanent this resolution animated; the DSL has no
        // effect that leaves a watch behind, and `Trigger::Dies` /
        // `Trigger::LeavesBattlefield` on this card would watch the Cub.
        // NOT SUPPORTED: "Whenever you tap a creature for mana, add an
        // additional {G}." — a triggered mana ability that adds to what the
        // tapped permanent's own ability produced. `Trigger::BecomesTapped`
        // fires on every tap (attacking, crew, convoke, a cost), and an
        // `Effect::AddMana` behind it would be an ordinary trigger on the
        // stack rather than "additional" mana.
    ],
);
