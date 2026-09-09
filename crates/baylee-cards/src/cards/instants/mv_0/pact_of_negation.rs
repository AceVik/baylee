//! Pact of Negation — {0} — Instant
//! Oracle: Counter target spell.
//! Oracle: At the beginning of your next upkeep, pay {3}{U}{U}. If you don't, you lose the game.
//! Set: TSR #77 — Time Spiral Remastered | Scryfall ID: 1ed4c0bb-b710-44a1-b8bc-6bd11c27b8b8 | Oracle ID: f3e213a4-ba5a-468a-93b3-c0a34e1bd725
// IMPLEMENTED — zero-cost counter + delayed pay-or-lose upkeep.

use baylee_cards_dsl::prelude::*;

card! {
    index: 107,
    oracle_id: "f3e213a4-ba5a-468a-93b3-c0a34e1bd725",
    scryfall_id: "1ed4c0bb-b710-44a1-b8bc-6bd11c27b8b8",
    faces: &[face! {
        name: "Pact of Negation",
        types: TypeSet::INSTANT,
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    coverage: Coverage::Implemented,
    abilities: &[spell!(&[
            Effect::CounterTargetSpell,
            Effect::PayCostOrLoseLater {
                cost: baylee_core::mana!("{3}{U}{U}"),
            },
        ], targets: Some(TargetReq::one(TargetSpec::Spell(&Filter::Any))))],
}
