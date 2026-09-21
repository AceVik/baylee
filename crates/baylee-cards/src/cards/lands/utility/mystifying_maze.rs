//! Mystifying Maze — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {4}, {T}: Exile target attacking creature an opponent controls. At the beginning of the next end step, return it to the battlefield tapped under its owner's control.
//! Set: C17 #264 — Commander 2017 | Scryfall ID: faeeb2f8-0aa1-4f3e-848d-f4233996055b | Oracle ID: 58bd67a8-1833-4827-aa33-1c141568f481
// PARTIAL — tap for {C} is implemented; exiling and returning tapped at next
// end step is not supported because ExileAndReturnAtEndStep returns untapped.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MYSTIFYING_MAZE,
    oracle_id = "58bd67a8-1833-4827-aa33-1c141568f481",
    scryfall_id = "faeeb2f8-0aa1-4f3e-848d-f4233996055b",
    coverage = Coverage::Partial(
        "ExileAndReturnAtEndStep returns the creature untapped rather than tapped",
    ),
    faces = &[face!(name = "Mystifying Maze", types = TypeSet::LAND,)],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: {4}, {T}: Exile target attacking creature an opponent controls. At the beginning of the next end step, return it to the battlefield tapped under its owner's control.
    ],
);
