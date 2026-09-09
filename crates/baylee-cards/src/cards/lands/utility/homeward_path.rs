//! Homeward Path — (no cost) — Land
//! Oracle: {T}: Add {C}. {T}: Each player gains control of all creatures they own.
//! Set: C13 #262 — Commander 2013 | Scryfall ID: 54734347-eee7-4c52-b514-7342afeccabd | Oracle ID: cb8ec2e4-8223-4172-8f2c-37c918a573fa
// IMPLEMENTED.

use baylee_cards_dsl::prelude::*;

card! {
    index: 71,
    oracle_id: "cb8ec2e4-8223-4172-8f2c-37c918a573fa",
    scryfall_id: "54734347-eee7-4c52-b514-7342afeccabd",
    faces: &[face! {
        name: "Homeward Path",
        types: TypeSet::LAND,
    }],
    coverage: Coverage::Implemented,
    abilities: &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(Cost::TAP, &[Effect::AllCreaturesToOwner]),
    ],
}
