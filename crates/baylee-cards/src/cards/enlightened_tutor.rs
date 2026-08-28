//! Enlightened Tutor — {W} — Instant
//! Oracle: Search your library for an artifact or enchantment card, reveal it, then shuffle and put that card on top.
//! Set: DMR #6 — Dominaria Remastered | Scryfall ID: 1c9675fb-1a89-420f-aea8-50e0642f549c | Oracle ID: c5229c17-b7be-4b05-b683-f2277edc4849
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(42),
    oracle_id: "c5229c17-b7be-4b05-b683-f2277edc4849",
    scryfall_id: "1c9675fb-1a89-420f-aea8-50e0642f549c",
    faces: &[FaceDef {
        name: "Enlightened Tutor",
        mana_cost: baylee_core::mana!("{W}"),
        types: TypeSet::INSTANT,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        power: None,
        toughness: None,
        loyalty: None,
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
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
