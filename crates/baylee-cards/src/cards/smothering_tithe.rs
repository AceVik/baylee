//! Smothering Tithe — {3}{W} — Enchantment
//! Oracle: Whenever an opponent draws a card, that player may pay {2}. If the player doesn't, you create a Treasure token. (It's an artifact with "{T}, Sacrifice this token: Add one mana of any color.")
//! Set: CMM #57 — Commander Masters | Scryfall ID: 861b5889-0183-4bee-afeb-a4b2aa700a8e | Oracle ID: 153376c9-dffd-458c-8ce3-a4c8269bc4e9
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(147),
    oracle_id: "153376c9-dffd-458c-8ce3-a4c8269bc4e9",
    scryfall_id: "861b5889-0183-4bee-afeb-a4b2aa700a8e",
    faces: &[FaceDef {
        name: "Smothering Tithe",
        mana_cost: baylee_core::mana!("{3}{W}"),
        types: TypeSet::ENCHANTMENT,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        power: None,
        toughness: None,
        loyalty: None,
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
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
