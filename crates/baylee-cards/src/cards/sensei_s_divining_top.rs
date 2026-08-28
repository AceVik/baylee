//! Sensei's Divining Top — {1} — Artifact
//! Oracle: {1}: Look at the top three cards of your library, then put them back in any order.
//! Oracle: {T}: Draw a card, then put this artifact on top of its owner's library.
//! Set: 2X2 #314 — Double Masters 2022 | Scryfall ID: e5142b7a-e580-4737-a4aa-2590f6610ceb | Oracle ID: 13575cf9-65c1-4861-b21e-eb2155e07766
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(142),
    oracle_id: "13575cf9-65c1-4861-b21e-eb2155e07766",
    scryfall_id: "e5142b7a-e580-4737-a4aa-2590f6610ceb",
    faces: &[FaceDef {
        name: "Sensei's Divining Top",
        mana_cost: baylee_core::mana!("{1}"),
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
