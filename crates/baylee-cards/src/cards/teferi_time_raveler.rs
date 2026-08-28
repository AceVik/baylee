//! Teferi, Time Raveler — {1}{W}{U} — Legendary Planeswalker — Teferi
//! Oracle: Each opponent can cast spells only any time they could cast a sorcery.
//! Oracle: +1: Until your next turn, you may cast sorcery spells as though they had flash.
//! Oracle: −3: Return up to one target artifact, creature, or enchantment to its owner's hand. Draw a card.
//! Set: RVR #232 — Ravnica Remastered | Scryfall ID: 662fe50f-d75c-422c-8c6c-1f9b5c4ba21f | Oracle ID: ae7604bb-4818-45a3-960c-cf3d83f15964
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(166),
    oracle_id: "ae7604bb-4818-45a3-960c-cf3d83f15964",
    scryfall_id: "662fe50f-d75c-422c-8c6c-1f9b5c4ba21f",
    faces: &[FaceDef {
        name: "Teferi, Time Raveler",
        mana_cost: baylee_core::mana!("{1}{W}{U}"),
        types: TypeSet::PLANESWALKER,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[subtypes::planeswalker::TEFERI],
        power: None,
        toughness: None,
        loyalty: Some(4),
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
