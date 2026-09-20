//! Gathering Place — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add {G} or {W}. Activate only if this land entered this turn or if you control a basic land.
//! Set: MSH #266 — Marvel Super Heroes | Scryfall ID: 081cdbd0-5081-4a9e-90ba-f5baf4ac137e | Oracle ID: 36b58705-c5a5-4547-8d8b-a7c35e1f69ae
// PARTIAL — {T}: Add {C}, and {T}: Add {G} or {W} gated on a basic land you
// control. The printed condition's other half — "if this land entered this
// turn" — has no Condition variant, so the disjunction is written as the
// basic-land half alone.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GATHERING_PLACE,
    oracle_id = "36b58705-c5a5-4547-8d8b-a7c35e1f69ae",
    scryfall_id = "081cdbd0-5081-4a9e-90ba-f5baf4ac137e",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    coverage = Coverage::Partial(
        "the coloured ability's condition is only half-sayable: no Condition \
         variant for \"this land entered this turn\""
    ),
    faces = &[face!(name = "Gathering Place", types = TypeSet::LAND,),],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: "…or if this land entered this turn" — the
        // disjunction is built from its basic-land clause only, so the land
        // cannot make {G} or {W} on the turn it enters.
        mana_ability!(
            Cost::TAP,
            &[Effect::mana_choice(&[ManaColor::Green, ManaColor::White])],
            condition = Some(Condition::ControlCount(&Filter::BASIC_LAND, 1))
        ),
    ],
);
