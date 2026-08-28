//! Elesh Norn, Mother of Machines — {4}{W} — Legendary Creature — Phyrexian Praetor
//! Oracle: Vigilance
//! Oracle: If a permanent entering causes a triggered ability of a permanent you control to trigger, that ability triggers an additional time.
//! Oracle: Permanents entering don't cause abilities of permanents your opponents control to trigger.
//! Set: ONE #10 — Phyrexia: All Will Be One | Scryfall ID: 44dcab01-1d13-4dfc-ae2f-fbaa3dd35087 | Oracle ID: 5ade11c0-41dd-4b6a-9f5b-c5903a3a0d7f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(39),
    oracle_id: "5ade11c0-41dd-4b6a-9f5b-c5903a3a0d7f",
    scryfall_id: "44dcab01-1d13-4dfc-ae2f-fbaa3dd35087",
    faces: &[FaceDef {
        name: "Elesh Norn, Mother of Machines",
        mana_cost: baylee_core::mana!("{4}{W}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[subtypes::creature::PHYREXIAN, subtypes::creature::PRAETOR],
        power: Some(4),
        toughness: Some(7),
        loyalty: None,
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::Legendary,
    partner: PartnerKind::None,
    coverage: Coverage::Unimplemented,
    abilities: &[],
};

#[cfg(test)]
mod tests {
    // TODO(card): implement abilities + tests, see docs/card-dsl.md.
}
