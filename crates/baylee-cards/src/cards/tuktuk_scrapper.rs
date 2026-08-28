//! Tuktuk Scrapper — {3}{R} — Creature — Goblin Artificer Ally
//! Oracle: Whenever this creature or another Ally you control enters, you may destroy target artifact. If that artifact is put into a graveyard this way, this creature deals damage to that artifact's controller equal to the number of Allies you control.
//! Set: WWK #94 — Worldwake | Scryfall ID: d3a84a2a-6384-497a-8ee2-de0fa74fcc80 | Oracle ID: 85cf2403-b419-4364-8ac9-67dd1ceddf9e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(174),
    oracle_id: "85cf2403-b419-4364-8ac9-67dd1ceddf9e",
    scryfall_id: "d3a84a2a-6384-497a-8ee2-de0fa74fcc80",
    faces: &[FaceDef {
        name: "Tuktuk Scrapper",
        mana_cost: baylee_core::mana!("{3}{R}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[
            subtypes::creature::GOBLIN,
            subtypes::creature::ARTIFICER,
            subtypes::creature::ALLY,
        ],
        power: Some(2),
        toughness: Some(2),
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
    }],
    color_identity: ColorSet::from_slice(&[Color::Red]),
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
