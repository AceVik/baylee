//! Gathering Place — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add {G} or {W}. Activate only if this land entered this turn or if you control a basic land.
//! Set: MSH #266 — Marvel Super Heroes | Scryfall ID: 081cdbd0-5081-4a9e-90ba-f5baf4ac137e | Oracle ID: 36b58705-c5a5-4547-8d8b-a7c35e1f69ae
// IMPLEMENTED — {T}: Add {C}, and {T}: Add {G} or {W} gated on the printed
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
    index = index::GATHERING_PLACE,
    oracle_id = "36b58705-c5a5-4547-8d8b-a7c35e1f69ae",
    scryfall_id = "081cdbd0-5081-4a9e-90ba-f5baf4ac137e",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(name = "Gathering Place", types = TypeSet::LAND,),],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(
            Cost::TAP,
            &[Effect::mana_choice(&[ManaColor::Green, ManaColor::White])],
            condition = Some(Condition::Any(&ENTERED_OR_BASIC))
        ),
    ],
);
