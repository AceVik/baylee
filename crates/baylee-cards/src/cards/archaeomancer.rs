//! Archaeomancer — {2}{U}{U} — Creature — Human Wizard
//! Oracle: When this creature enters, return target instant or sorcery card from your graveyard to your hand.
//! Set: UMA #45 — Ultimate Masters | Scryfall ID: cc258713-6ce3-44e0-9b4b-8fa7d1d093a1 | Oracle ID: a91a3266-cadd-47a0-9b20-160307f14c07
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(6),
    oracle_id: "a91a3266-cadd-47a0-9b20-160307f14c07",
    scryfall_id: "cc258713-6ce3-44e0-9b4b-8fa7d1d093a1",
    faces: &[FaceDef {
        name: "Archaeomancer",
        mana_cost: baylee_core::mana!("{2}{U}{U}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power: Some(1),
        toughness: Some(2),
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
