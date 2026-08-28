//! Mulldrifter — {4}{U} — Creature — Elemental
//! Oracle: Flying
//! Oracle: When this creature enters, draw two cards.
//! Oracle: Evoke {2}{U} (You may cast this spell for its evoke cost. If you do, it's sacrificed when it enters.)
//! Set: ECC #67 — Lorwyn Eclipsed Commander | Scryfall ID: 3de308cc-14ac-407e-99e7-568572ecd0e7 | Oracle ID: 24d0f5e7-0d9e-4b76-900e-a7274e80312d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(99),
    oracle_id: "24d0f5e7-0d9e-4b76-900e-a7274e80312d",
    scryfall_id: "3de308cc-14ac-407e-99e7-568572ecd0e7",
    faces: &[FaceDef {
        name: "Mulldrifter",
        mana_cost: baylee_core::mana!("{4}{U}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[subtypes::creature::ELEMENTAL],
        power: Some(2),
        toughness: Some(2),
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
