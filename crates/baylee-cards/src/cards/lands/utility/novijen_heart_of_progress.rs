//! Novijen, Heart of Progress — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {G}{U}, {T}: Put a +1/+1 counter on each creature that entered this turn.
//! Set: C21 #305 — Commander 2021 | Scryfall ID: 9a1e15e7-4ba6-41ad-b27b-aee2d037b6a7 | Oracle ID: b3b5137d-0225-4dba-9231-d235ab0f137c
// IMPLEMENTED — {T}: Add {C}, and {G}{U}, {T} for a +1/+1 counter on every
// creature that entered this turn.

use baylee_cards_dsl::prelude::*;

/// "each creature that entered this turn" — every creature on the
/// battlefield and not only this seat's, which is what the printing says
/// and what a `ControlledByYou` beside it would quietly take away.
static ARRIVED: Filter = Filter::And(&[Filter::CREATURE, Filter::EnteredThisTurn]);

card!(
    index = index::NOVIJEN_HEART_OF_PROGRESS,
    oracle_id = "b3b5137d-0225-4dba-9231-d235ab0f137c",
    scryfall_id = "9a1e15e7-4ba6-41ad-b27b-aee2d037b6a7",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Blue]),
    faces = &[face!(
        name = "Novijen, Heart of Progress",
        types = TypeSet::LAND,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{G}{U}", TapSelf),
            &[Effect::AddCounterFilter {
                filter: &ARRIVED,
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(1),
            }],
        ),
    ],
);
