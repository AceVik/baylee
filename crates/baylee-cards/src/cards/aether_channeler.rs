//! Aether Channeler — {2}{U} — Creature — Human Wizard
//! Oracle: When this creature enters, choose one —
//! Oracle: • Create a 1/1 white Bird creature token with flying.
//! Oracle: • Return another target nonland permanent to its owner's hand.
//! Oracle: • Draw a card.
//! Set: DMU #42 — Dominaria United | Scryfall ID: 60afeb75-2c1e-4634-8c83-88b1dddb77c2 | Oracle ID: fb220f46-f8b8-4804-baa4-e7d50b4871f7
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(2),
    oracle_id: "fb220f46-f8b8-4804-baa4-e7d50b4871f7",
    scryfall_id: "60afeb75-2c1e-4634-8c83-88b1dddb77c2",
    faces: &[FaceDef {
        name: "Aether Channeler",
        mana_cost: baylee_core::mana!("{2}{U}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power: Some(2),
        toughness: Some(1),
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
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
