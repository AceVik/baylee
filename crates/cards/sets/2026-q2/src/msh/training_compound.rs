//! Training Compound — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add {R} or {G}. Activate only if this land entered this turn or if you control a basic land.
//! Set: MSH #275 — Marvel Super Heroes | Scryfall ID: c91e28db-307f-462a-88aa-581d10e77f10 | Oracle ID: 99c70f4e-de8a-426d-99aa-17b2f87625ba
// IMPLEMENTED — {T}: Add {C}, and {T}: Add {R} or {G} gated on the printed
// disjunction: this land entered this turn, or you control a basic land.

use baylee_cards_dsl::prelude::*;

/// "…or if this land entered this turn" — Gathering Place's sentence, and
/// its two questions joined the same way.
static ENTERED_OR_BASIC: [Condition; 2] = [
    Condition::SourceMatches(&Filter::EnteredThisTurn),
    Condition::ControlCount(&Filter::BASIC_LAND, 1),
];

card!(
    index = index::TRAINING_COMPOUND,
    oracle_id = "99c70f4e-de8a-426d-99aa-17b2f87625ba",
    scryfall_id = "c91e28db-307f-462a-88aa-581d10e77f10",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(name = "Training Compound", types = TypeSet::LAND,),],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(
            Cost::TAP,
            &[Effect::mana_choice(&[ManaColor::Red, ManaColor::Green])],
            condition = Some(Condition::Any(&ENTERED_OR_BASIC))
        ),
    ],
);
