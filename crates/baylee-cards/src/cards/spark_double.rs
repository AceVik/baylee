//! Spark Double — {3}{U} — Creature — Illusion
//! Oracle: You may have this creature enter as a copy of a creature or planeswalker you control, except it enters with an additional +1/+1 counter on it if it's a creature, it enters with an additional loyalty counter on it if it's a planeswalker, and it isn't legendary.
//! Set: RVR #62 — Ravnica Remastered | Scryfall ID: c41b9ba2-0006-4d8e-b600-efe81ff5e0cc | Oracle ID: 8dcb35e5-ae44-455f-86e3-4a77d496ff34
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(154),
    oracle_id: "8dcb35e5-ae44-455f-86e3-4a77d496ff34",
    scryfall_id: "c41b9ba2-0006-4d8e-b600-efe81ff5e0cc",
    faces: &[FaceDef {
        name: "Spark Double",
        mana_cost: baylee_core::mana!("{3}{U}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[subtypes::creature::ILLUSION],
        power: Some(0),
        toughness: Some(0),
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
