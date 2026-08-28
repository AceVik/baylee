//! Force of Will — {3}{U}{U} — Instant
//! Oracle: You may pay 1 life and exile a blue card from your hand rather than pay this spell's mana cost.
//! Oracle: Counter target spell.
//! Set: DMR #50 — Dominaria Remastered | Scryfall ID: 89f612d6-7c59-4a7b-a87d-45f789e88ba5 | Oracle ID: 956381ba-6d37-4a8a-846c-bad79222dbee
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(54),
    oracle_id: "956381ba-6d37-4a8a-846c-bad79222dbee",
    scryfall_id: "89f612d6-7c59-4a7b-a87d-45f789e88ba5",
    faces: &[FaceDef {
        name: "Force of Will",
        mana_cost: baylee_core::mana!("{3}{U}{U}"),
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
