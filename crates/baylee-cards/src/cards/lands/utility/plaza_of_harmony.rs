//! Plaza of Harmony — (no cost) — Land
//! Oracle: When this land enters, if you control two or more Gates, you gain 3 life.
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add one mana of any type that a Gate you control could produce.
//! Set: RNA #254 — Ravnica Allegiance | Scryfall ID: 07f2ae00-206d-4984-84eb-d10ab75d3791 | Oracle ID: 5ff1d6d8-8cea-4a25-90d9-b575f4c99bc8
// PARTIAL — the enter trigger with its two-Gate intervening `if`, and the {C}
// ability; the Gate-restricted mana line has no spelling in the DSL.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

card!(
    index = index::PLAZA_OF_HARMONY,
    oracle_id = "5ff1d6d8-8cea-4a25-90d9-b575f4c99bc8",
    scryfall_id = "07f2ae00-206d-4984-84eb-d10ab75d3791",
    faces = &[face!(name = "Plaza of Harmony", types = TypeSet::LAND,),],
    coverage =
        Coverage::Partial("ManaSource cannot name \"a mana a Gate you control could produce\""),
    abilities = &[
        triggered!(
            Trigger::ETB,
            &[Effect::gain_life(3)],
            condition = Some(Condition::ControlCount(&Filter::HasSubtype(land::GATE), 2))
        ),
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: {T}: Add one mana of any type that a Gate you control
        // could produce — the nearest variant, Effect::mana_land_color(true),
        // reads "a land you control" and carries no filter, so it would hand
        // out a colour from any land. The ability comes off rather than
        // shipping mana the card does not print.
    ],
);
