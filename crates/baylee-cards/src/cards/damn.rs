//! Damn — {B}{B} — Sorcery
//! Oracle: Destroy target creature. A creature destroyed this way can't be regenerated.
//! Oracle: Overload {2}{W}{W} (You may cast this spell for its overload cost. If you do, change "target" in its text to "each.")
//! Set: LCC #191 — The Lost Caverns of Ixalan Commander | Scryfall ID: 84056124-1a6f-4274-bee2-74cf0debddb5 | Oracle ID: b01d61cc-9844-4191-86a0-f2db6d42d6e5
// IMPLEMENTED — single-target destroy or overloaded wrath. ("A creature
// destroyed this way can't be regenerated" is vacuous: the engine has no
// regeneration mechanic yet; noted for the roadmap's regeneration family.)

static NORMAL_EFFECTS: &[Effect] = &[Effect::Destroy {
    target: TargetSpec::Object(&Filter::CREATURE),
}];
static OVERLOAD_EFFECTS: &[Effect] = &[Effect::DestroyAll {
    filter: &Filter::CREATURE,
}];

use baylee_cards_dsl::prelude::*;

card! {
    index: 30,
    oracle_id: "b01d61cc-9844-4191-86a0-f2db6d42d6e5",
    scryfall_id: "84056124-1a6f-4274-bee2-74cf0debddb5",
    faces: &[face! {
        name: "Damn",
        mana_cost: baylee_core::mana!("{B}{B}"),
        types: TypeSet::SORCERY,
    }],
    color_identity: ColorSet::from_slice(&[Color::Black, Color::White]),
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::ModalSpell {
        modes: &[
            mode!(NORMAL_EFFECTS, target: Some(TargetSpec::Object(&Filter::CREATURE))),
            mode!(OVERLOAD_EFFECTS, cost_override: Some(baylee_core::mana!("{2}{W}{W}"))),
        ],
    }],
}

// Overload destroys everything; normal mode only the target.
