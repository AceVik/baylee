//! Fierce Guardianship — {2}{U} — Instant
//! Oracle: If you control a commander, you may cast this spell without paying its mana cost.
//! Oracle: Counter target noncreature spell.
//! Set: CMM #94 — Commander Masters | Scryfall ID: f7f3dd95-bd14-4e0f-a388-444f9cf1b0dc | Oracle ID: d09c9cba-fdd2-479b-ad5d-d05181c3e3f9
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(50),
    oracle_id: "d09c9cba-fdd2-479b-ad5d-d05181c3e3f9",
    scryfall_id: "f7f3dd95-bd14-4e0f-a388-444f9cf1b0dc",
    faces: &[FaceDef {
        name: "Fierce Guardianship",
        mana_cost: baylee_core::mana!("{2}{U}"),
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
