//! Spellseeker — {2}{U} — Creature — Human Wizard
//! Oracle: When this creature enters, you may search your library for an instant or sorcery card with mana value 2 or less, reveal it, put it into your hand, then shuffle.
//! Set: CMM #120 — Commander Masters | Scryfall ID: a749c591-2fbe-41d8-ac5b-56ebce82d33e | Oracle ID: 47a785ed-8095-4685-8daa-02c4e2b0ffcd
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(155),
    oracle_id: "47a785ed-8095-4685-8daa-02c4e2b0ffcd",
    scryfall_id: "a749c591-2fbe-41d8-ac5b-56ebce82d33e",
    faces: &[FaceDef {
        name: "Spellseeker",
        mana_cost: baylee_core::mana!("{2}{U}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power: Some(1),
        toughness: Some(1),
        loyalty: None,
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
