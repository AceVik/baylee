//! Gleaming Bastion — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add {W} or {U}. Activate only if this land entered this turn or if you control a basic land.
//! Set: MSH #267 — Marvel Super Heroes | Scryfall ID: c9131bf5-17e3-4aa6-97ed-ed6426b247d0 | Oracle ID: 7785ffd4-f169-475d-9558-ce4877b3378a
// PARTIAL — {C} unconditionally, and the {W}/{U} ability gated on the
// printed "or if you control a basic land" half of its condition; the
// "entered this turn" half has no vocabulary.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GLEAMING_BASTION,
    oracle_id = "7785ffd4-f169-475d-9558-ce4877b3378a",
    scryfall_id = "c9131bf5-17e3-4aa6-97ed-ed6426b247d0",
    color_identity = ColorSet::from_slice(&[Color::Blue, Color::White]),
    faces = &[face!(name = "Gleaming Bastion", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "the \"or if this land entered this turn\" half of the {W}/{U} ability's \
         activation condition: no Condition names a permanent that entered this turn, \
         and Condition has no Or to join it to the basic-land half"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: "or if this land entered this turn" — the gate is the
        // basic-land half alone, so a Bastion played this turn with no other
        // basic land under its controller is refused mana it may print.
        mana_ability!(
            Cost::TAP,
            &[Effect::mana_choice(&[ManaColor::White, ManaColor::Blue])],
            condition = Some(Condition::ControlCount(&Filter::BASIC_LAND, 1))
        ),
    ],
);
