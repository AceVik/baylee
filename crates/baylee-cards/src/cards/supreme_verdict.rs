//! Supreme Verdict — {1}{W}{W}{U} — Sorcery
//! Oracle: This spell can't be countered.
//! Oracle: Destroy all creatures.
//! Set: CLU #211 — Ravnica: Clue Edition | Scryfall ID: 3892f1c5-937e-4ef4-b6f9-e0c0ded070d0 | Oracle ID: 0230de18-8d15-4cfa-9d42-7ccddd9f9570
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(160),
    oracle_id: "0230de18-8d15-4cfa-9d42-7ccddd9f9570",
    scryfall_id: "3892f1c5-937e-4ef4-b6f9-e0c0ded070d0",
    faces: &[FaceDef {
        name: "Supreme Verdict",
        mana_cost: baylee_core::mana!("{1}{W}{W}{U}"),
        types: TypeSet::SORCERY,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        power: None,
        toughness: None,
        loyalty: None,
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
