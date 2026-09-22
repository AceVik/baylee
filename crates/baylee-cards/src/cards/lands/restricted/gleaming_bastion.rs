//! Gleaming Bastion — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add {W} or {U}. Activate only if this land entered this turn or if you control a basic land.
//! Set: MSH #267 — Marvel Super Heroes | Scryfall ID: c9131bf5-17e3-4aa6-97ed-ed6426b247d0 | Oracle ID: 7785ffd4-f169-475d-9558-ce4877b3378a
// IMPLEMENTED — {T}: Add {C}, and {T}: Add {W} or {U} gated on the printed
// disjunction: this land entered this turn, or you control a basic land.

use baylee_cards_dsl::prelude::*;

/// "…or if this land entered this turn". Two questions of different kinds —
/// one reads this land's own history, the other counts the board — which is
/// why `Condition::Any` joins them instead of either variant growing a
/// second clause.
static ENTERED_OR_BASIC: [Condition; 2] = [
    Condition::SourceMatches(&Filter::EnteredThisTurn),
    Condition::ControlCount(&Filter::BASIC_LAND, 1),
];

card!(
    index = index::GLEAMING_BASTION,
    oracle_id = "7785ffd4-f169-475d-9558-ce4877b3378a",
    scryfall_id = "c9131bf5-17e3-4aa6-97ed-ed6426b247d0",
    color_identity = ColorSet::from_slice(&[Color::Blue, Color::White]),
    faces = &[face!(name = "Gleaming Bastion", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(
            Cost::TAP,
            &[Effect::mana_choice(&[ManaColor::White, ManaColor::Blue])],
            condition = Some(Condition::Any(&ENTERED_OR_BASIC))
        ),
    ],
);
