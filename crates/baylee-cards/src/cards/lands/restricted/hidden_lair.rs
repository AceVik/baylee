//! Hidden Lair — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add {U} or {B}. Activate only if this land entered this turn or if you control a basic land.
//! Set: MSH #269 — Marvel Super Heroes | Scryfall ID: 0742ddb6-71ed-444e-91ad-84f876725a4a | Oracle ID: 7069d241-4e66-40bf-afd1-551a4a5457f0
// PARTIAL — {C} always; {U}/{B} written with the "you control a basic land"
// half of its condition only, because nothing in the DSL can say "this land
// entered this turn".

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HIDDEN_LAIR,
    oracle_id = "7069d241-4e66-40bf-afd1-551a4a5457f0",
    scryfall_id = "0742ddb6-71ed-444e-91ad-84f876725a4a",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue]),
    faces = &[face!(name = "Hidden Lair", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "the {U}/{B} ability's \"if this land entered this turn\" half: no Filter or Condition names a turn of entry, so only the basic-land half is asked"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: "or if this land entered this turn" — entering this
        // turn is not a characteristic, so no Filter and no Condition can
        // carry it; the ability therefore asks only "if you control a basic
        // land" and is offered in fewer situations than the card prints.
        mana_ability!(
            Cost::TAP,
            &[Effect::mana_choice(&[ManaColor::Blue, ManaColor::Black])],
            condition = Some(Condition::ControlCount(&Filter::BASIC_LAND, 1))
        ),
    ],
);
