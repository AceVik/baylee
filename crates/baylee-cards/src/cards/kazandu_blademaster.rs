//! Kazandu Blademaster — {W}{W} — Creature — Human Soldier Ally
//! Oracle: First strike, vigilance
//! Oracle: Whenever this creature or another Ally you control enters, you may put a +1/+1 counter on this creature.
//! Set: ZEN #16 — Zendikar | Scryfall ID: 9642bdbf-c03f-4c48-a5c8-c9201a08b834 | Oracle ID: 133f5d30-d883-493e-93a1-cf9583db460b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(83),
    oracle_id: "133f5d30-d883-493e-93a1-cf9583db460b",
    scryfall_id: "9642bdbf-c03f-4c48-a5c8-c9201a08b834",
    faces: &[FaceDef {
        name: "Kazandu Blademaster",
        mana_cost: baylee_core::mana!("{W}{W}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[
            subtypes::creature::HUMAN,
            subtypes::creature::SOLDIER,
            subtypes::creature::ALLY,
        ],
        power: Some(1),
        toughness: Some(1),
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Unimplemented,
    abilities: &[],
};

#[cfg(test)]
mod tests {
    // TODO(card): implement abilities + tests, see docs/card-dsl.md.
}
