//! Helm of the Host — {4} — Legendary Artifact — Equipment
//! Oracle: At the beginning of combat on your turn, create a token that's a copy of equipped creature, except the token isn't legendary. That token gains haste.
//! Oracle: Equip {5}
//! Set: MSC #200 — Marvel Super Heroes Commander | Scryfall ID: 70ffc71f-328d-421d-926b-6f2e45ffb812 | Oracle ID: 83b43aba-bf9c-4da2-967d-9daa632e97d2
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(68),
    oracle_id: "83b43aba-bf9c-4da2-967d-9daa632e97d2",
    scryfall_id: "70ffc71f-328d-421d-926b-6f2e45ffb812",
    faces: &[FaceDef {
        name: "Helm of the Host",
        mana_cost: baylee_core::mana!("{4}"),
        types: TypeSet::ARTIFACT,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[subtypes::artifact::EQUIPMENT],
        power: None,
        toughness: None,
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
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
