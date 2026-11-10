//! Oran-Rief, the Vastwood — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {G}.
//! Oracle: {T}: Put a +1/+1 counter on each green creature that entered this turn.
//! Set: SOC #391 — Secrets of Strixhaven Commander | Scryfall ID: d46d3ef7-b81f-42f4-9c1b-fa53f11eae65 | Oracle ID: e88027a6-24cc-4a8b-86db-734f26149ea8
// IMPLEMENTED — enters tapped, and taps for {G}. The third ability is refused
// rather than approximated: nothing in `Filter` says "entered this turn".

use baylee_cards_dsl::prelude::*;

/// "each green creature that entered this turn", everybody's — the colour
/// is read off the projection, so an anthem that made a creature green
/// puts it under this land exactly as a printed green one is.
static ARRIVED_GREEN: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasColor(ColorSet::from_slice(&[Color::Green])),
    Filter::EnteredThisTurn,
]);

card!(
    index = index::ORAN_RIEF_THE_VASTWOOD,
    oracle_id = "e88027a6-24cc-4a8b-86db-734f26149ea8",
    scryfall_id = "d46d3ef7-b81f-42f4-9c1b-fa53f11eae65",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Oran-Rief, the Vastwood",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Green, 1)]),
        activated!(
            Cost::TAP,
            &[Effect::AddCounterFilter {
                filter: &ARRIVED_GREEN,
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(1),
            }],
        ),
    ],
);
