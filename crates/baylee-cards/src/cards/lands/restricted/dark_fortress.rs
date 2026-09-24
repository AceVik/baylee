//! Dark Fortress — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add {B} or {R}. Activate only if this land entered this turn or if you control a basic land.
//! Set: MSH #264 — Marvel Super Heroes | Scryfall ID: c16fd43c-7c47-4c1b-860f-91146532e89d | Oracle ID: 40760bfa-a423-487c-ba29-043b2d00c736
// IMPLEMENTED — {T}: Add {C}, and {T}: Add {B} or {R} gated on the printed
// disjunction: this land entered this turn, or you control a basic land.

use baylee_cards_dsl::prelude::*;

/// "…or if this land entered this turn" — Gathering Place's sentence, and
/// its two questions joined the same way.
static ENTERED_OR_BASIC: [Condition; 2] = [
    Condition::SourceMatches(&Filter::EnteredThisTurn),
    Condition::ControlCount(&Filter::BASIC_LAND, 1),
];

card!(
    index = index::DARK_FORTRESS,
    oracle_id = "40760bfa-a423-487c-ba29-043b2d00c736",
    scryfall_id = "c16fd43c-7c47-4c1b-860f-91146532e89d",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(name = "Dark Fortress", types = TypeSet::LAND,),],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(
            Cost::TAP,
            &[Effect::mana_choice(&[ManaColor::Black, ManaColor::Red])],
            condition = Some(Condition::Any(&ENTERED_OR_BASIC))
        ),
    ],
);
