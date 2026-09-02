//! Mana Drain — {U}{U} — Instant
//! Oracle: Counter target spell. At the beginning of your next main phase, add an amount of {C} equal to that spell's mana value.
//! Set: 2X2 #57 — Double Masters 2022 | Scryfall ID: 3c429c40-2389-41e5-8681-4bb274e25eba | Oracle ID: 74d3277a-38e5-4732-afed-084a56148f20
// IMPLEMENTED — counter + delayed colorless mana at your next first main.

use baylee_cards_dsl::prelude::*;

card! {
    index: 90,
    oracle_id: "74d3277a-38e5-4732-afed-084a56148f20",
    scryfall_id: "3c429c40-2389-41e5-8681-4bb274e25eba",
    faces: &[face! {
        name: "Mana Drain",
        mana_cost: baylee_core::mana!("{U}{U}"),
        types: TypeSet::INSTANT,
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    coverage: Coverage::Implemented,
    abilities: &[spell!(&[
            Effect::CounterTargetSpell,
            Effect::DelayedManaAtNextFirstMain {
                color: ManaColor::Colorless,
            },
        ], targets: Some(TargetReq::one(TargetSpec::Spell(&Filter::Any))))],
}
