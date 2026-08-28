//! Liquimetal Torque — {2} — Artifact
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Target nonland permanent becomes an artifact in addition to its other types until end of turn.
//! Set: MH2 #228 — Modern Horizons 2 | Scryfall ID: 13c6101a-da40-4785-8ccb-4e779bbbdb55 | Oracle ID: b7d4b7dd-fbb1-4ca3-875f-ef13a95e66ad
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(86),
    oracle_id: "b7d4b7dd-fbb1-4ca3-875f-ef13a95e66ad",
    scryfall_id: "13c6101a-da40-4785-8ccb-4e779bbbdb55",
    faces: &[FaceDef {
        name: "Liquimetal Torque",
        mana_cost: baylee_core::mana!("{2}"),
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
