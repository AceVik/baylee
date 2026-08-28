//! Opposition Agent — {2}{B} — Creature — Human Rogue
//! Oracle: Flash
//! Oracle: You control your opponents while they're searching their libraries.
//! Oracle: While an opponent is searching their library, they exile each card they find. You may play those cards for as long as they remain exiled, and you may spend mana as though it were mana of any color to cast them.
//! Set: CMR #141 — Commander Legends | Scryfall ID: 086f97e9-8b62-44f3-b467-149c2ac5ca78 | Oracle ID: 1f438b8f-fe23-4f3b-ab2e-f6c33676c462
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(105),
    oracle_id: "1f438b8f-fe23-4f3b-ab2e-f6c33676c462",
    scryfall_id: "086f97e9-8b62-44f3-b467-149c2ac5ca78",
    faces: &[FaceDef {
        name: "Opposition Agent",
        mana_cost: baylee_core::mana!("{2}{B}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[subtypes::creature::HUMAN, subtypes::creature::ROGUE],
        power: Some(3),
        toughness: Some(2),
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
    }],
    color_identity: ColorSet::from_slice(&[Color::Black]),
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
