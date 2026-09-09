//! Doubling Season — {4}{G} — Enchantment
//! Oracle: If an effect would create one or more tokens under your control, it creates twice that many of those tokens instead.
//! Oracle: If an effect would put one or more counters on a permanent you control, it puts twice that many of those counters on that permanent instead.
//! Set: FDN #216 — Foundations | Scryfall ID: f2c4f80e-84a0-463b-82c3-5c6503809351 | Oracle ID: 01546b7d-a233-4176-8843-d732074dc5b6
// IMPLEMENTED — token creation and counter placement (every counter kind on a
// permanent), including a planeswalker's starting loyalty (CR 306.5b/614.16).

use baylee_cards_dsl::prelude::*;

card! {
    index: 35,
    oracle_id: "01546b7d-a233-4176-8843-d732074dc5b6",
    scryfall_id: "f2c4f80e-84a0-463b-82c3-5c6503809351",
    faces: &[face! {
        name: "Doubling Season",
        mana_cost: baylee_core::mana!("{4}{G}"),
        types: TypeSet::ENCHANTMENT,
    }],
    color_identity: ColorSet::from_slice(&[Color::Green]),
    coverage: Coverage::Implemented,
    abilities: &[
        AbilityDef::Replacement(ReplacementRule::DoubleTokenCreation {
            controller_filter: &Filter::ControlledByYou,
        }),
        AbilityDef::Replacement(ReplacementRule::DoubleCounterPlacement {
            object_filter: &Filter::ControlledByYou,
        }),
    ],
}

// Engine-level coverage in baylee-engine s6 tests: Maskwood Nexus's
// token ability creates two Shapeshifters with Doubling Season out.
