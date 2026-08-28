//! Karn, the Great Creator — {4} — Legendary Planeswalker — Karn
//! Oracle: Activated abilities of artifacts your opponents control can't be activated.
//! Oracle: +1: Until your next turn, up to one target noncreature artifact becomes an artifact creature with power and toughness each equal to its mana value.
//! Oracle: −2: You may reveal an artifact card you own from outside the game or choose a face-up artifact card you own in exile. Put that card into your hand.
//! Set: RVR #1 — Ravnica Remastered | Scryfall ID: deb3721d-fba1-444f-8b31-1cd10c94c4a0 | Oracle ID: a20dd48d-d344-4db1-b0e9-a2b71c3cc9d1
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(81),
    oracle_id: "a20dd48d-d344-4db1-b0e9-a2b71c3cc9d1",
    scryfall_id: "deb3721d-fba1-444f-8b31-1cd10c94c4a0",
    faces: &[FaceDef {
        name: "Karn, the Great Creator",
        mana_cost: baylee_core::mana!("{4}"),
        types: TypeSet::PLANESWALKER,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[subtypes::planeswalker::KARN],
        power: None,
        toughness: None,
        loyalty: Some(5),
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
    }],
    color_identity: ColorSet::EMPTY,
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
