//! Primaris Eliminator — {4}{B} — Creature — Astartes Warrior
//! Oracle: When this creature enters, choose one —
//! Oracle: • Executioner Round — Destroy target creature.
//! Oracle: • Hyperfrag Round — Creatures target player controls get -2/-2 until end of turn.
//! Set: 40K #50 — Warhammer 40,000 Commander | Scryfall ID: db7ab081-d6cd-4323-98bf-536e4df95115 | Oracle ID: 7d679591-f8ea-4c4c-ab98-7b9e3438cf57
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(118),
    oracle_id: "7d679591-f8ea-4c4c-ab98-7b9e3438cf57",
    scryfall_id: "db7ab081-d6cd-4323-98bf-536e4df95115",
    faces: &[FaceDef {
        name: "Primaris Eliminator",
        mana_cost: baylee_core::mana!("{4}{B}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[subtypes::creature::ASTARTES, subtypes::creature::WARRIOR],
        power: Some(3),
        toughness: Some(2),
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
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
