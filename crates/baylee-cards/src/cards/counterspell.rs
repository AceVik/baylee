//! Counterspell — {U}{U} — Instant
//! Oracle: Counter target spell.
//! Set: DSC #114 — Duskmourn: House of Horror Commander | Scryfall ID: 4f616706-ec97-4923-bb1e-11a69fbaa1f8 | Oracle ID: cc187110-1148-4090-bbb8-e205694a39f5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(25),
    oracle_id: "cc187110-1148-4090-bbb8-e205694a39f5",
    scryfall_id: "4f616706-ec97-4923-bb1e-11a69fbaa1f8",
    faces: &[FaceDef {
        name: "Counterspell",
        mana_cost: baylee_core::mana!("{U}{U}"),
        types: TypeSet::INSTANT,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        power: None,
        toughness: None,
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
