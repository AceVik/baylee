//! Sakashima of a Thousand Faces — {3}{U} — Legendary Creature — Human Rogue
//! Oracle: You may have Sakashima enter as a copy of another creature you control, except it has Sakashima's other abilities.
//! Oracle: The "legend rule" doesn't apply to permanents you control.
//! Oracle: Partner (You can have two commanders if both have partner.)
//! Set: CMR #89 — Commander Legends | Scryfall ID: 714c3a1f-7b30-4ed8-8f38-6176758741fb | Oracle ID: 8ecdaf4b-4442-42da-9714-4257a83faf50
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(137),
    oracle_id: "8ecdaf4b-4442-42da-9714-4257a83faf50",
    scryfall_id: "714c3a1f-7b30-4ed8-8f38-6176758741fb",
    faces: &[FaceDef {
        name: "Sakashima of a Thousand Faces",
        mana_cost: baylee_core::mana!("{3}{U}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[subtypes::creature::HUMAN, subtypes::creature::ROGUE],
        power: Some(3),
        toughness: Some(1),
        loyalty: None,
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::Legendary,
    partner: PartnerKind::Partner,
    coverage: Coverage::Unimplemented,
    abilities: &[],
};

#[cfg(test)]
mod tests {
    // TODO(card): implement abilities + tests, see docs/card-dsl.md.
}
