//! Nesting Dovehawk — {3}{W} — Creature — Bird
//! Oracle: Flying
//! Oracle: At the beginning of combat on your turn, populate. (Create a token that's a copy of a creature token you control.)
//! Oracle: Whenever a creature token you control enters, put a +1/+1 counter on this creature.
//! Set: MOC #17 — March of the Machine Commander | Scryfall ID: c58ff93f-7135-40af-92ce-358da48694dc | Oracle ID: fe8fc442-ed17-40b2-8624-69f2eed3f9be
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(103),
    oracle_id: "fe8fc442-ed17-40b2-8624-69f2eed3f9be",
    scryfall_id: "c58ff93f-7135-40af-92ce-358da48694dc",
    faces: &[FaceDef {
        name: "Nesting Dovehawk",
        mana_cost: baylee_core::mana!("{3}{W}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[subtypes::creature::BIRD],
        power: Some(2),
        toughness: Some(2),
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
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
