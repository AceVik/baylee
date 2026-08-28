//! Damn — {B}{B} — Sorcery
//! Oracle: Destroy target creature. A creature destroyed this way can't be regenerated.
//! Oracle: Overload {2}{W}{W} (You may cast this spell for its overload cost. If you do, change "target" in its text to "each.")
//! Set: LCC #191 — The Lost Caverns of Ixalan Commander | Scryfall ID: 84056124-1a6f-4274-bee2-74cf0debddb5 | Oracle ID: b01d61cc-9844-4191-86a0-f2db6d42d6e5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(30),
    oracle_id: "b01d61cc-9844-4191-86a0-f2db6d42d6e5",
    scryfall_id: "84056124-1a6f-4274-bee2-74cf0debddb5",
    faces: &[FaceDef {
        name: "Damn",
        mana_cost: baylee_core::mana!("{B}{B}"),
        types: TypeSet::SORCERY,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        power: None,
        toughness: None,
        loyalty: None,
    }],
    color_identity: ColorSet::from_slice(&[Color::Black, Color::White]),
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
