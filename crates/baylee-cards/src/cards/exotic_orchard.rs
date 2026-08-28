//! Exotic Orchard — (no cost) — Land
//! Oracle: {T}: Add one mana of any color that a land an opponent controls could produce.
//! Set: MBC #79 — Mystery Booster Commander Edition | Scryfall ID: d11c5fe0-1528-4c94-a8cc-42bcab9d7487 | Oracle ID: 27b047e3-0d41-45e2-98e9-9391d7923a1e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(48),
    oracle_id: "27b047e3-0d41-45e2-98e9-9391d7923a1e",
    scryfall_id: "d11c5fe0-1528-4c94-a8cc-42bcab9d7487",
    faces: &[FaceDef {
        name: "Exotic Orchard",
        mana_cost: ManaCost::ZERO,
        types: TypeSet::LAND,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        power: None,
        toughness: None,
        loyalty: None,
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
