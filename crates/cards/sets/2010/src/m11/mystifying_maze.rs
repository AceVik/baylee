//! Mystifying Maze — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {4}, {T}: Exile target attacking creature an opponent controls. At the beginning of the next end step, return it to the battlefield tapped under its owner's control.
//! Set: C17 #264 — Commander 2017 | Scryfall ID: faeeb2f8-0aa1-4f3e-848d-f4233996055b | Oracle ID: 58bd67a8-1833-4827-aa33-1c141568f481
use baylee_cards_dsl::prelude::*;

card!(
    index = index::MYSTIFYING_MAZE,
    oracle_id = "58bd67a8-1833-4827-aa33-1c141568f481",
    scryfall_id = "faeeb2f8-0aa1-4f3e-848d-f4233996055b",
    coverage = Coverage::Implemented,
    faces = &[face!(name = "Mystifying Maze", types = TypeSet::LAND,)],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{4}", TapSelf),
            &[Effect::ExileAndReturnTappedAtEndStep],
            target = Some(TargetSpec::Object(&Filter::And(&[
                Filter::CREATURE,
                Filter::Attacking,
                Filter::ControlledByOpponent
            ])))
        ),
    ],
);
