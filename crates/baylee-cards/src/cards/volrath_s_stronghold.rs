//! Volrath's Stronghold — (no cost) — Legendary Land
//! Oracle: {T}: Add {C}.
//! Oracle: {1}{B}, {T}: Put target creature card from your graveyard on top of your library.
//! Set: TPR #248 — Tempest Remastered | Scryfall ID: f465ae5f-61f0-42c4-978f-841ba1226f56 | Oracle ID: 73b8cf90-3c71-4f8b-a29f-61894b7f27c9
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(186),
    oracle_id: "73b8cf90-3c71-4f8b-a29f-61894b7f27c9",
    scryfall_id: "f465ae5f-61f0-42c4-978f-841ba1226f56",
    faces: &[FaceDef {
        name: "Volrath's Stronghold",
        mana_cost: ManaCost::ZERO,
        types: TypeSet::LAND,
        supertypes: SupertypeSet::LEGENDARY,
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
