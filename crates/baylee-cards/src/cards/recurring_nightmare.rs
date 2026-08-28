//! Recurring Nightmare — {2}{B} — Enchantment
//! Oracle: Sacrifice a creature, Return this enchantment to its owner's hand: Return target creature card from your graveyard to the battlefield. Activate only as a sorcery.
//! Set: TPR #113 — Tempest Remastered | Scryfall ID: b50e1800-a45c-43bd-8886-8a06145d9346 | Oracle ID: a6708b11-1bcd-4208-a967-fe91f2e3313c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(127),
    oracle_id: "a6708b11-1bcd-4208-a967-fe91f2e3313c",
    scryfall_id: "b50e1800-a45c-43bd-8886-8a06145d9346",
    faces: &[FaceDef {
        name: "Recurring Nightmare",
        mana_cost: baylee_core::mana!("{2}{B}"),
        types: TypeSet::ENCHANTMENT,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        power: None,
        toughness: None,
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
    }],
    color_identity: ColorSet::from_slice(&[Color::Black]),
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
