//! Mulldrifter — {4}{U} — Creature — Elemental
//! Oracle: Flying
//! Oracle: When this creature enters, draw two cards.
//! Oracle: Evoke {2}{U} (You may cast this spell for its evoke cost. If you do, it's sacrificed when it enters.)
//! Set: ECC #67 — Lorwyn Eclipsed Commander | Scryfall ID: 3de308cc-14ac-407e-99e7-568572ecd0e7 | Oracle ID: 24d0f5e7-0d9e-4b76-900e-a7274e80312d
// IMPLEMENTED — evoke (alternative cost + sacrifice on ETB when evoked).

use baylee_cards_dsl::prelude::*;

card! {
    index: 99,
    oracle_id: "24d0f5e7-0d9e-4b76-900e-a7274e80312d",
    scryfall_id: "3de308cc-14ac-407e-99e7-568572ecd0e7",
    faces: &[face! {
        name: "Mulldrifter",
        mana_cost: baylee_core::mana!("{4}{U}"),
        types: TypeSet::CREATURE,
        subtypes: &[baylee_core::generated::subtypes::creature::ELEMENTAL],
        power: Some(2),
        toughness: Some(2),
        alternative_costs: &[AlternativeCost {
            cost: Cost {
                mana: baylee_core::mana!("{2}{U}"),
                parts: &[],
            },
            condition: AltCondition::Always,
        }],
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    keywords: KeywordSet::FLYING,
    coverage: Coverage::Implemented,
    abilities: &[
        triggered!(Trigger::EntersBattlefield(&Filter::This), &[Effect::DrawCards {
                amount: Amount::Fixed(2),
            }]),
        triggered!(Trigger::EntersBattlefieldEvoked, &[Effect::SacrificeSelf]),
    ],
}

// Evoke path: cast for {2}{U}, ETB draws 2, then it is sacrificed.
// Full path: cast for {4}{U}, it stays.
