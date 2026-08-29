//! Mulldrifter — {4}{U} — Creature — Elemental
//! Oracle: Flying
//! Oracle: When this creature enters, draw two cards.
//! Oracle: Evoke {2}{U} (You may cast this spell for its evoke cost. If you do, it's sacrificed when it enters.)
//! Set: ECC #67 — Lorwyn Eclipsed Commander | Scryfall ID: 3de308cc-14ac-407e-99e7-568572ecd0e7 | Oracle ID: 24d0f5e7-0d9e-4b76-900e-a7274e80312d
// IMPLEMENTED — evoke (alternative cost + sacrifice on ETB when evoked).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, AltCondition, AlternativeCost, Amount, CardDef, CommanderRule, Cost, Coverage,
    Effect, FaceDef, Filter, KeywordSet, PartnerKind, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(99),
    oracle_id: "24d0f5e7-0d9e-4b76-900e-a7274e80312d",
    scryfall_id: "3de308cc-14ac-407e-99e7-568572ecd0e7",
    faces: &[FaceDef {
        name: "Mulldrifter",
        mana_cost: baylee_core::mana!("{4}{U}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[baylee_core::generated::subtypes::creature::ELEMENTAL],
        power: Some(2),
        toughness: Some(2),
        loyalty: None,
        alternative_costs: &[AlternativeCost {
            cost: Cost {
                mana: baylee_core::mana!("{2}{U}"),
                parts: &[],
            },
            condition: AltCondition::Always,
        }],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
        abilities: &[],
        castable_from_hand: true,
        miracle: None,
        delve: false,
        convoke: false,
        cost_reduction: None,
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    keywords: KeywordSet::FLYING,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[
        AbilityDef::Triggered {
            trigger: Trigger::EntersBattlefield(&Filter::This),
            once_per_turn: false,
            effects: &[Effect::DrawCards {
                amount: Amount::Fixed(2),
            }],
            targets: None,
        },
        AbilityDef::Triggered {
            trigger: Trigger::EntersBattlefieldEvoked,
            once_per_turn: false,
            effects: &[Effect::SacrificeSelf],
            targets: None,
        },
    ],
};

#[cfg(test)]
mod tests {
    // Evoke path: cast for {2}{U}, ETB draws 2, then it is sacrificed.
    // Full path: cast for {4}{U}, it stays.
}
