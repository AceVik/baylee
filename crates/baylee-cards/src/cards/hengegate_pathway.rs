//! Hengegate Pathway // Mistgate Pathway — (no cost) — Land // Land
//! Set: KHM #260 — Kaldheim | Scryfall ID: 7ef37cb3-d803-47d7-8a01-9c803aa2eadc | Oracle ID: 461b3f2f-fcee-4160-abfa-061f8b6a784f
//! Face: Hengegate Pathway —  — Land
//! Face: Mistgate Pathway —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(69),
    oracle_id: "461b3f2f-fcee-4160-abfa-061f8b6a784f",
    scryfall_id: "7ef37cb3-d803-47d7-8a01-9c803aa2eadc",
    faces: &[
        FaceDef {
            name: "Hengegate Pathway",
            mana_cost: ManaCost::ZERO,
            types: TypeSet::LAND,
            supertypes: SupertypeSet::EMPTY,
            subtypes: &[],
            power: None,
            toughness: None,
            loyalty: None,
        },
        FaceDef {
            name: "Mistgate Pathway",
            mana_cost: ManaCost::ZERO,
            types: TypeSet::LAND,
            supertypes: SupertypeSet::EMPTY,
            subtypes: &[],
            power: None,
            toughness: None,
            loyalty: None,
        },
    ],
    color_identity: ColorSet::from_slice(&[Color::Blue, Color::White]),
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
