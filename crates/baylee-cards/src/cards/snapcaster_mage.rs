//! Snapcaster Mage — {1}{U} — Creature — Human Wizard
//! Oracle: Flash
//! Oracle: When this creature enters, target instant or sorcery card in your graveyard gains flashback until end of turn. The flashback cost is equal to its mana cost. (You may cast that card from your graveyard for its flashback cost. Then exile it.)
//! Set: INR #478 — Innistrad Remastered | Scryfall ID: 22b36ad5-bf4d-436a-9c3c-fa4acd0052fe | Oracle ID: 2bb2eda7-3b38-4c56-870f-c3218a1056f5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(148),
    oracle_id: "2bb2eda7-3b38-4c56-870f-c3218a1056f5",
    scryfall_id: "22b36ad5-bf4d-436a-9c3c-fa4acd0052fe",
    faces: &[FaceDef {
        name: "Snapcaster Mage",
        mana_cost: baylee_core::mana!("{1}{U}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power: Some(2),
        toughness: Some(1),
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
        abilities: &[],
        castable_from_hand: true,
        miracle: None,
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
