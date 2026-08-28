//! Fracture — {W}{B} — Instant
//! Oracle: Destroy target artifact, enchantment, or planeswalker.
//! Set: SOC #310 — Secrets of Strixhaven Commander | Scryfall ID: cba33bf7-0919-408c-8eb0-0bb9fe920c81 | Oracle ID: f21d0319-0509-4ac1-b6e3-10955a26fd7a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(56),
    oracle_id: "f21d0319-0509-4ac1-b6e3-10955a26fd7a",
    scryfall_id: "cba33bf7-0919-408c-8eb0-0bb9fe920c81",
    faces: &[FaceDef {
        name: "Fracture",
        mana_cost: baylee_core::mana!("{W}{B}"),
        types: TypeSet::INSTANT,
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
    color_identity: ColorSet::from_slice(&[Color::Black, Color::White]),
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
