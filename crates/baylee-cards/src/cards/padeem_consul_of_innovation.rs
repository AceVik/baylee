//! Padeem, Consul of Innovation — {3}{U} — Legendary Creature — Vedalken Artificer
//! Oracle: Artifacts you control have hexproof. (They can't be the targets of spells or abilities your opponents control.)
//! Oracle: At the beginning of your upkeep, if you control the artifact with the greatest mana value or tied for the greatest mana value, draw a card.
//! Set: CMM #109 — Commander Masters | Scryfall ID: 00a4aef8-64fc-4e9d-adac-ef4c85d40b4a | Oracle ID: 0c7ba712-6a99-4d2f-9242-a2163a11f69c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(108),
    oracle_id: "0c7ba712-6a99-4d2f-9242-a2163a11f69c",
    scryfall_id: "00a4aef8-64fc-4e9d-adac-ef4c85d40b4a",
    faces: &[FaceDef {
        name: "Padeem, Consul of Innovation",
        mana_cost: baylee_core::mana!("{3}{U}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[subtypes::creature::VEDALKEN, subtypes::creature::ARTIFICER],
        power: Some(1),
        toughness: Some(4),
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::Legendary,
    partner: PartnerKind::None,
    coverage: Coverage::Unimplemented,
    abilities: &[],
};

#[cfg(test)]
mod tests {
    // TODO(card): implement abilities + tests, see docs/card-dsl.md.
}
