//! Homeward Path — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Each player gains control of all creatures they own.
//! Set: C16 #301 — Commander 2016 | Scryfall ID: 54734347-eee7-4c52-b514-7342afeccabd | Oracle ID: cb8ec2e4-8223-4172-8f2c-37c918a573fa
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(71),
    oracle_id: "cb8ec2e4-8223-4172-8f2c-37c918a573fa",
    scryfall_id: "54734347-eee7-4c52-b514-7342afeccabd",
    faces: &[FaceDef {
        name: "Homeward Path",
        mana_cost: ManaCost::ZERO,
        types: TypeSet::LAND,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        power: None,
        toughness: None,
        loyalty: None,
    }],
    color_identity: ColorSet::EMPTY,
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
