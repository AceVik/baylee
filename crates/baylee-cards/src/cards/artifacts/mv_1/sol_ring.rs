//! Sol Ring — {1} — Artifact
//! Oracle: {T}: Add {C}{C}.
//! Set: MSC #211 — Marvel Super Heroes Commander | Scryfall ID: 91fdb56b-54d5-4272-8319-505ff987fe9b | Oracle ID: 6ad8011d-3471-4369-9d68-b264cc027487
// IMPLEMENTED — mana rock (mana ability, resolves without the stack).

use baylee_cards_dsl::prelude::*;

card! {
    index: 150,
    oracle_id: "6ad8011d-3471-4369-9d68-b264cc027487",
    scryfall_id: "91fdb56b-54d5-4272-8319-505ff987fe9b",
    faces: &[face! {
        name: "Sol Ring",
        mana_cost: baylee_core::mana!("{1}"),
        types: TypeSet::ARTIFACT,
    }],
    coverage: Coverage::Implemented,
    abilities: &[mana_ability!(&[Effect::mana(ManaColor::Colorless, 2)])],
}

// Engine-level coverage via s4 scenario tests: tapping adds {C}{C}
// immediately (no stack object created).
