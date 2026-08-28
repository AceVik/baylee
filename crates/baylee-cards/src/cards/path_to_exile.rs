//! Path to Exile — {W} — Instant
//! Oracle: Exile target creature. Its controller may search their library for a basic land card, put that card onto the battlefield tapped, then shuffle.
//! Set: MSC #141 — Marvel Super Heroes Commander | Scryfall ID: 95ca89ea-1200-4bb4-ae4b-af35d3ccd35b | Oracle ID: d683d985-9888-4d21-8b5f-69e69ce4a03b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(112),
    oracle_id: "d683d985-9888-4d21-8b5f-69e69ce4a03b",
    scryfall_id: "95ca89ea-1200-4bb4-ae4b-af35d3ccd35b",
    faces: &[FaceDef {
        name: "Path to Exile",
        mana_cost: baylee_core::mana!("{W}"),
        types: TypeSet::INSTANT,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        power: None,
        toughness: None,
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
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
