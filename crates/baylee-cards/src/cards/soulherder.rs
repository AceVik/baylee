//! Soulherder — {1}{W}{U} — Creature — Spirit
//! Oracle: Whenever a creature is exiled from the battlefield, put a +1/+1 counter on this creature.
//! Oracle: At the beginning of your end step, you may exile another target creature you control, then return that card to the battlefield under its owner's control.
//! Set: KHC #93 — Kaldheim Commander | Scryfall ID: 50bc0f5b-7421-45b9-af85-86dd9821b7d8 | Oracle ID: 92019547-f6db-4ea6-8356-d0a90ace5662
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(153),
    oracle_id: "92019547-f6db-4ea6-8356-d0a90ace5662",
    scryfall_id: "50bc0f5b-7421-45b9-af85-86dd9821b7d8",
    faces: &[FaceDef {
        name: "Soulherder",
        mana_cost: baylee_core::mana!("{1}{W}{U}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[subtypes::creature::SPIRIT],
        power: Some(1),
        toughness: Some(1),
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
    }],
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
