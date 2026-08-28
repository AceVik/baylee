//! Storm of Saruman — {4}{U}{U} — Enchantment
//! Oracle: Ward {3}
//! Oracle: Whenever you cast your second spell each turn, copy it, except the copy isn't legendary. You may choose new targets for the copy. (A copy of a permanent spell becomes a token.)
//! Set: LTR #72 — The Lord of the Rings: Tales of Middle-earth | Scryfall ID: 52884e67-c742-4799-9afd-55bc70b2cf40 | Oracle ID: cf5f4860-e805-46a3-9352-a2c583e33403
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(157),
    oracle_id: "cf5f4860-e805-46a3-9352-a2c583e33403",
    scryfall_id: "52884e67-c742-4799-9afd-55bc70b2cf40",
    faces: &[FaceDef {
        name: "Storm of Saruman",
        mana_cost: baylee_core::mana!("{4}{U}{U}"),
        types: TypeSet::ENCHANTMENT,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        power: None,
        toughness: None,
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
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
