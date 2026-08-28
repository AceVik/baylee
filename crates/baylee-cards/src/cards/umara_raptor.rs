//! Umara Raptor — {2}{U} — Creature — Bird Ally
//! Oracle: Flying
//! Oracle: Whenever this creature or another Ally you control enters, you may put a +1/+1 counter on this creature.
//! Set: ZEN #75 — Zendikar | Scryfall ID: 6049cc80-1faa-48bf-897e-fefe5a8e7ab2 | Oracle ID: a58ee84f-1d9c-4924-b7b1-14a9b2ba3b98
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(177),
    oracle_id: "a58ee84f-1d9c-4924-b7b1-14a9b2ba3b98",
    scryfall_id: "6049cc80-1faa-48bf-897e-fefe5a8e7ab2",
    faces: &[FaceDef {
        name: "Umara Raptor",
        mana_cost: baylee_core::mana!("{2}{U}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[subtypes::creature::BIRD, subtypes::creature::ALLY],
        power: Some(1),
        toughness: Some(1),
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
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
