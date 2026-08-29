//! Tower of the Magistrate — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {1}, {T}: Target creature gains protection from artifacts until end of turn.
//! Set: MMQ #330 — Mercadian Masques | Scryfall ID: ee0481db-15ae-46b4-89a3-01c95a9626c7 | Oracle ID: ac08fae8-208c-4602-8d39-9bfd29b53a5e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(171),
    oracle_id: "ac08fae8-208c-4602-8d39-9bfd29b53a5e",
    scryfall_id: "ee0481db-15ae-46b4-89a3-01c95a9626c7",
    faces: &[FaceDef {
        name: "Tower of the Magistrate",
        mana_cost: ManaCost::ZERO,
        types: TypeSet::LAND,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        power: None,
        toughness: None,
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
        abilities: &[],
        castable_from_hand: true,
        miracle: None,
    }],
    color_identity: ColorSet::EMPTY,
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
