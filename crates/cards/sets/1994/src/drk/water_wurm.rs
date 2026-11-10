//! Water Wurm — {U} — Creature — Wurm
//! Oracle: This creature gets +0/+1 as long as an opponent controls an Island.
//! Set: DRK #38 — The Dark | Scryfall ID: e3da4a88-5225-467f-9240-f30bc1eee520 | Oracle ID: cfa373c4-6e42-4cf3-94a4-841f2824603e

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const OPPONENT_ISLAND: Filter = Filter::And(&[
    Filter::ControlledByOpponent,
    Filter::HasSubtype(subtypes::land::ISLAND),
]);

card!(
    index = index::WATER_WURM,
    oracle_id = "cfa373c4-6e42-4cf3-94a4-841f2824603e",
    scryfall_id = "e3da4a88-5225-467f-9240-f30bc1eee520",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Water Wurm",
        mana_cost = mana!("{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WURM],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[static_ability!(
        Filter::This,
        Modifier::ModifyPT(0, 1),
        condition = Some(Condition::OpponentControlCount(&OPPONENT_ISLAND, 1)),
    )],
);
