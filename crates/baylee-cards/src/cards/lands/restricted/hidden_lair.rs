//! Hidden Lair — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add {U} or {B}. Activate only if this land entered this turn or if you control a basic land.
//! Set: MSH #269 — Marvel Super Heroes | Scryfall ID: 0742ddb6-71ed-444e-91ad-84f876725a4a | Oracle ID: 7069d241-4e66-40bf-afd1-551a4a5457f0
// IMPLEMENTED — {T}: Add {C}, and {T}: Add {U} or {B} gated on the printed
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
    index = index::HIDDEN_LAIR,
    oracle_id = "7069d241-4e66-40bf-afd1-551a4a5457f0",
    scryfall_id = "0742ddb6-71ed-444e-91ad-84f876725a4a",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue]),
    faces = &[face!(name = "Hidden Lair", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(
            Cost::TAP,
            &[Effect::mana_choice(&[ManaColor::Blue, ManaColor::Black])],
            condition = Some(Condition::Any(&ENTERED_OR_BASIC))
        ),
    ],
);
