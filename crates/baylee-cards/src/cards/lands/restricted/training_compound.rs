//! Training Compound — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add {R} or {G}. Activate only if this land entered this turn or if you control a basic land.
//! Set: MSH #275 — Marvel Super Heroes | Scryfall ID: c91e28db-307f-462a-88aa-581d10e77f10 | Oracle ID: 99c70f4e-de8a-426d-99aa-17b2f87625ba
// PARTIAL — "{T}: Add {C}" is written; the conditional "{R}/{G}" ability is
// off the card, because its printed condition is a disjunction.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TRAINING_COMPOUND,
    oracle_id = "99c70f4e-de8a-426d-99aa-17b2f87625ba",
    scryfall_id = "c91e28db-307f-462a-88aa-581d10e77f10",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Red]),
    faces = &[face!(name = "Training Compound", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "{T}: Add {R} or {G} is off the card — its printed condition is an \"or\" over two sentences and Condition has no disjunction"
    ),
    // NOT SUPPORTED: "{T}: Add {R} or {G}. Activate only if this land entered
    // this turn or if you control a basic land." — `Condition` carries five
    // sentences and no `Or` over them, so the `ControlCount(&Filter::BASIC_LAND, 1)`
    // half alone would refuse the ability on the turn the land itself arrived
    // (the printed card allows it), and no `Filter` can say "entered this
    // turn": a permanent keeps no record of when it arrived, so
    // `Condition::SourceMatches` has nothing to point at.
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)])],
);
