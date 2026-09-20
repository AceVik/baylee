//! Glimmervoid — (no cost) — Land
//! Oracle: At the beginning of the end step, if you control no artifacts, sacrifice this land.
//! Oracle: {T}: Add one mana of any color.
//! Set: 2XM #319 — Double Masters | Scryfall ID: 4a639687-d9e3-46a8-bc9f-6ca3912c46ab | Oracle ID: b92e9854-4527-4133-8615-e282a213e7e3
// PARTIAL — {T}: Add one mana of any color; the end-step sacrifice has no
// condition in the DSL.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GLIMMERVOID,
    oracle_id = "b92e9854-4527-4133-8615-e282a213e7e3",
    scryfall_id = "4a639687-d9e3-46a8-bc9f-6ca3912c46ab",
    faces = &[face!(name = "Glimmervoid", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "no Condition says \"you control no artifacts\": ControlCount counts a minimum"
    ),
    // NOT SUPPORTED: "At the beginning of the end step, if you control no
    // artifacts, sacrifice this land" — the only counted condition is
    // Condition::ControlCount(&filter, n), which is "at least n" (metalcraft,
    // the verge lands), and no effect variant branches on a control count
    // either, so the printed sentence has no spelling here.
    abilities = &[mana_ability!(&[Effect::mana_of_any_color()])],
);
