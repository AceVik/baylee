//! Ruins of Oran-Rief — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {C}. ({C} represents colorless mana.)
//! Oracle: {T}: Put a +1/+1 counter on target colorless creature that entered this turn.
//! Set: CMM #1023 — Commander Masters | Scryfall ID: d1159ef6-f3ac-42a0-ae46-7d5eb9b3a6eb | Oracle ID: 7140f396-1bfa-4b28-ba28-fa15eba74652
// IMPLEMENTED — enters tapped, {T}: Add {C}, and the +1/+1 counter on a
// colorless creature that entered this turn.

use baylee_cards_dsl::prelude::*;

/// "target colorless creature that entered this turn" — three clauses, and
/// the third is history rather than a characteristic. `Filter::IsColorless`
/// reads the projected colours, so a creature an effect has made colourless
/// is on the menu exactly as a printed one is.
static ARRIVED_COLORLESS: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::IsColorless,
    Filter::EnteredThisTurn,
]);

card!(
    index = index::RUINS_OF_ORAN_RIEF,
    oracle_id = "7140f396-1bfa-4b28-ba28-fa15eba74652",
    scryfall_id = "d1159ef6-f3ac-42a0-ae46-7d5eb9b3a6eb",
    faces = &[face!(
        name = "Ruins of Oran-Rief",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            Cost::TAP,
            &[Effect::AddCounter {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(1),
            }],
            target = Some(TargetSpec::Object(&ARRIVED_COLORLESS)),
        ),
    ],
);
