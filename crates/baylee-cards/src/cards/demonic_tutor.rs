//! Demonic Tutor — {1}{B} — Sorcery
//! Oracle: Search your library for a card, put that card into your hand, then shuffle.
//! Set: CMM #150 — Commander Masters | Scryfall ID: a24b4cb6-cebb-428b-8654-74347a6a8d63 | Oracle ID: 82004860-e589-4e38-8d61-8c0210e4ea39
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(32),
    oracle_id: "82004860-e589-4e38-8d61-8c0210e4ea39",
    scryfall_id: "a24b4cb6-cebb-428b-8654-74347a6a8d63",
    faces: &[FaceDef {
        name: "Demonic Tutor",
        mana_cost: baylee_core::mana!("{1}{B}"),
        types: TypeSet::SORCERY,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        power: None,
        toughness: None,
        loyalty: None,
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
