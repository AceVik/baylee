//! Oran-Rief, the Vastwood — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {G}.
//! Oracle: {T}: Put a +1/+1 counter on each green creature that entered this turn.
//! Set: SOC #391 — Secrets of Strixhaven Commander | Scryfall ID: d46d3ef7-b81f-42f4-9c1b-fa53f11eae65 | Oracle ID: e88027a6-24cc-4a8b-86db-734f26149ea8
// IMPLEMENTED — enters tapped, and taps for {G}. The third ability is refused
// rather than approximated: nothing in `Filter` says "entered this turn".

use baylee_cards_dsl::prelude::*;

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
    coverage = Coverage::Partial(
        "{T}: Put a +1/+1 counter on each green creature that entered this turn — no Filter variant says \"entered this turn\""
    ),
    abilities = &[
        // NOT SUPPORTED: "{T}: Put a +1/+1 counter on each green creature that
        // entered this turn." The counter half is Effect::AddCounterFilter and
        // the green half is a two-clause filter, but "entered this turn" is a
        // history predicate no Filter carries — written as a plain green
        // creature it would counter the whole board, so the ability comes off.
        mana_ability!(&[Effect::mana(ManaColor::Green, 1)]),
    ],
);
