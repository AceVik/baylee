//! Kor Haven — (no cost) — Legendary Land
//! Oracle: {T}: Add {C}.
//! Oracle: {1}{W}, {T}: Prevent all combat damage that would be dealt by target attacking creature this turn.
//! Set: NEM #141 — Nemesis | Scryfall ID: 3d5529ca-5c20-4dfd-8595-96d6dfa6debe | Oracle ID: 276cece9-f9f2-46e6-ae76-daddaa2fb9ab
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(84),
    oracle_id: "276cece9-f9f2-46e6-ae76-daddaa2fb9ab",
    scryfall_id: "3d5529ca-5c20-4dfd-8595-96d6dfa6debe",
    faces: &[FaceDef {
        name: "Kor Haven",
        mana_cost: ManaCost::ZERO,
        types: TypeSet::LAND,
        supertypes: SupertypeSet::LEGENDARY,
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
