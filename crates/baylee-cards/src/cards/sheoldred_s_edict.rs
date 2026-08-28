//! Sheoldred's Edict — {1}{B} — Instant
//! Oracle: Choose one —
//! Oracle: • Each opponent sacrifices a nontoken creature of their choice.
//! Oracle: • Each opponent sacrifices a creature token of their choice.
//! Oracle: • Each opponent sacrifices a planeswalker of their choice.
//! Set: ONE #108 — Phyrexia: All Will Be One | Scryfall ID: a9225cc3-90f0-448f-a8d9-7c6c2796d077 | Oracle ID: 217062f5-96f1-454c-9507-17f34ef37070
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(144),
    oracle_id: "217062f5-96f1-454c-9507-17f34ef37070",
    scryfall_id: "a9225cc3-90f0-448f-a8d9-7c6c2796d077",
    faces: &[FaceDef {
        name: "Sheoldred's Edict",
        mana_cost: baylee_core::mana!("{1}{B}"),
        types: TypeSet::INSTANT,
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
