//! Eerie Interlude — {2}{W} — Instant
//! Oracle: Exile any number of target creatures you control. Return those cards to the battlefield under their owner's control at the beginning of the next end step.
//! Set: KHC #22 — Kaldheim Commander | Scryfall ID: 4ba9f15f-00d2-4797-9228-91b320e85705 | Oracle ID: 0634091a-a74c-4cea-b6d1-7324a725554a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(38),
    oracle_id: "0634091a-a74c-4cea-b6d1-7324a725554a",
    scryfall_id: "4ba9f15f-00d2-4797-9228-91b320e85705",
    faces: &[FaceDef {
        name: "Eerie Interlude",
        mana_cost: baylee_core::mana!("{2}{W}"),
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
