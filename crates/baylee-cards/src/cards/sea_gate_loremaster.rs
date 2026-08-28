//! Sea Gate Loremaster — {4}{U} — Creature — Merfolk Wizard Ally
//! Oracle: {T}: Draw a card for each Ally you control.
//! Set: ZEN #63 — Zendikar | Scryfall ID: 5cd723c8-4b3d-4fbb-a825-79934279382d | Oracle ID: 6eed122b-9760-47fd-8ba2-adeda8054e0d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(141),
    oracle_id: "6eed122b-9760-47fd-8ba2-adeda8054e0d",
    scryfall_id: "5cd723c8-4b3d-4fbb-a825-79934279382d",
    faces: &[FaceDef {
        name: "Sea Gate Loremaster",
        mana_cost: baylee_core::mana!("{4}{U}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[
            subtypes::creature::MERFOLK,
            subtypes::creature::WIZARD,
            subtypes::creature::ALLY,
        ],
        power: Some(1),
        toughness: Some(3),
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
