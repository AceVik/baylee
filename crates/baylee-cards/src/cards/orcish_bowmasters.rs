//! Orcish Bowmasters — {1}{B} — Creature — Orc Archer
//! Oracle: Flash
//! Oracle: When this creature enters and whenever an opponent draws a card except the first one they draw in each of their draw steps, this creature deals 1 damage to any target. Then amass Orcs 1.
//! Set: LTR #103 — The Lord of the Rings: Tales of Middle-earth | Scryfall ID: 7c024bae-5631-4e20-ac69-df392ac9e109 | Oracle ID: ea5103f5-27e0-4eb1-902c-7f34652d6bf3
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(106),
    oracle_id: "ea5103f5-27e0-4eb1-902c-7f34652d6bf3",
    scryfall_id: "7c024bae-5631-4e20-ac69-df392ac9e109",
    faces: &[FaceDef {
        name: "Orcish Bowmasters",
        mana_cost: baylee_core::mana!("{1}{B}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[subtypes::creature::ORC, subtypes::creature::ARCHER],
        power: Some(1),
        toughness: Some(1),
        loyalty: None,
    }],
    color_identity: ColorSet::from_slice(&[Color::Black]),
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
