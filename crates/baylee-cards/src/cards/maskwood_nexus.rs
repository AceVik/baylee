//! Maskwood Nexus — {4} — Artifact
//! Oracle: Creatures you control are every creature type. The same is true for creature spells you control and creature cards you own that aren't on the battlefield.
//! Oracle: {3}, {T}: Create a 2/2 blue Shapeshifter creature token with changeling. (It is every creature type.)
//! Set: CLB #865 — Commander Legends: Battle for Baldur's Gate | Scryfall ID: 1246c42d-57c0-4cba-959a-15ad89d8a50b | Oracle ID: 9b2cdbed-c733-409b-b0e4-2c8960c25111
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(92),
    oracle_id: "9b2cdbed-c733-409b-b0e4-2c8960c25111",
    scryfall_id: "1246c42d-57c0-4cba-959a-15ad89d8a50b",
    faces: &[FaceDef {
        name: "Maskwood Nexus",
        mana_cost: baylee_core::mana!("{4}"),
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
