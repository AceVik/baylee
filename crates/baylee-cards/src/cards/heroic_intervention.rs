//! Heroic Intervention — {1}{G} — Instant
//! Oracle: Permanents you control gain hexproof and indestructible until end of turn.
//! Set: CMM #295 — Commander Masters | Scryfall ID: e32c67d1-187f-40df-b3b3-6036f5c92834 | Oracle ID: 24882fa2-3fe9-4c1b-aa3d-0e6488b9db27
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(70),
    oracle_id: "24882fa2-3fe9-4c1b-aa3d-0e6488b9db27",
    scryfall_id: "e32c67d1-187f-40df-b3b3-6036f5c92834",
    faces: &[FaceDef {
        name: "Heroic Intervention",
        mana_cost: baylee_core::mana!("{1}{G}"),
        types: TypeSet::INSTANT,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        power: None,
        toughness: None,
        loyalty: None,
    }],
    color_identity: ColorSet::from_slice(&[Color::Green]),
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
