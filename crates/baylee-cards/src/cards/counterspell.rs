//! Counterspell — {U}{U} — Instant
//! Oracle: Counter target spell.
//! Set: DSC #114 — Duskmourn: House of Horror Commander | Scryfall ID: 4f616706-ec97-4923-bb1e-11a69fbaa1f8 | Oracle ID: cc187110-1148-4090-bbb8-e205694a39f5
// IMPLEMENTED — hard counter (target selection on the stack).

use baylee_cards_dsl::prelude::*;

card! {
    index: 25,
    oracle_id: "cc187110-1148-4090-bbb8-e205694a39f5",
    scryfall_id: "4f616706-ec97-4923-bb1e-11a69fbaa1f8",
    faces: &[face! {
        name: "Counterspell",
        mana_cost: baylee_core::mana!("{U}{U}"),
        types: TypeSet::INSTANT,
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    coverage: Coverage::Implemented,
    abilities: &[spell!(&[Effect::CounterTargetSpell], targets: Some(TargetReq::one(TargetSpec::Spell(&Filter::Any))))],
}

// Engine-level coverage via s4 scenario tests: countering a creature
// spell moves it to the graveyard instead of the battlefield.
