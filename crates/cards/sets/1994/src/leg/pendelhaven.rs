//! Pendelhaven — (no cost) — Legendary Land
//! Oracle: {T}: Add {G}.
//! Oracle: {T}: Target 1/1 creature gets +1/+2 until end of turn.
//! Set: A25 #244 — Masters 25 | Scryfall ID: acf85879-4d14-4d86-978c-b155c47b7dcd | Oracle ID: f70e72e1-9abe-485b-9fea-e8b35352f5b3
// IMPLEMENTED — {T}: Add {G}, and the pump through the exact 1/1 restriction
// the card prints on its target.

use baylee_cards_dsl::prelude::*;

/// "1/1" is an **exact** size and therefore four predicates, not two: a
/// bound in one direction alone admits the creature the printing refuses —
/// `ToughnessAtMost(1)` takes a 2/1 and `PowerAtLeast(1)` takes a 3/3. The
/// pair per characteristic is what says *equal to*, and there is no
/// `PowerExactly`, because these two already spell it and a third variant
/// would be a second way to say one thing.
///
/// It is a restriction on the **target** and so a `TargetSpec` filter: CR
/// 115.3 makes a creature that is not 1/1 an illegal target, so it is never
/// offered, and CR 608.2b takes the ability off the stack if the creature
/// stops being 1/1 before it resolves — which is the whole of what the card
/// does when an anthem lands in response.
static ONE_ONE_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::PowerAtLeast(1),
    Filter::PowerAtMost(1),
    Filter::ToughnessAtLeast(1),
    Filter::ToughnessAtMost(1),
]);

card!(
    index = index::PENDELHAVEN,
    oracle_id = "f70e72e1-9abe-485b-9fea-e8b35352f5b3",
    scryfall_id = "acf85879-4d14-4d86-978c-b155c47b7dcd",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Pendelhaven",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Green, 1)]),
        activated!(
            Cost::TAP,
            &[Effect::PumpTarget {
                power: Amount::Fixed(1),
                toughness: Amount::Fixed(2),
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn,
            }],
            target = Some(TargetSpec::Object(&ONE_ONE_CREATURE)),
        ),
    ],
);
