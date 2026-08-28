//! Chromatic Lantern — {3} — Artifact
//! Oracle: Lands you control have "{T}: Add one mana of any color."
//! Oracle: {T}: Add one mana of any color.
//! Set: MBC #73 — Mystery Booster Commander Edition | Scryfall ID: 9b29492a-8bdd-4806-8d1b-3058ed277cc1 | Oracle ID: 539f5396-d99a-417d-a84c-dff7930b5900
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(19),
    oracle_id: "539f5396-d99a-417d-a84c-dff7930b5900",
    scryfall_id: "9b29492a-8bdd-4806-8d1b-3058ed277cc1",
    faces: &[FaceDef {
        name: "Chromatic Lantern",
        mana_cost: baylee_core::mana!("{3}"),
        types: TypeSet::ARTIFACT,
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
