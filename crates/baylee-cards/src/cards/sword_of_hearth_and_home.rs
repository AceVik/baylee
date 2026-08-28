//! Sword of Hearth and Home — {3} — Artifact — Equipment
//! Oracle: Equipped creature gets +2/+2 and has protection from green and from white.
//! Oracle: Whenever equipped creature deals combat damage to a player, exile up to one target creature you own, then search your library for a basic land card. Put both cards onto the battlefield under your control, then shuffle.
//! Oracle: Equip {2}
//! Set: MH2 #238 — Modern Horizons 2 | Scryfall ID: a16fabbe-4557-4067-b882-f2e5dbd8b458 | Oracle ID: 913e6182-706a-4872-8c8a-e146b0ae0738
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(163),
    oracle_id: "913e6182-706a-4872-8c8a-e146b0ae0738",
    scryfall_id: "a16fabbe-4557-4067-b882-f2e5dbd8b458",
    faces: &[FaceDef {
        name: "Sword of Hearth and Home",
        mana_cost: baylee_core::mana!("{3}"),
        types: TypeSet::ARTIFACT,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[subtypes::artifact::EQUIPMENT],
        power: None,
        toughness: None,
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
    }],
    color_identity: ColorSet::EMPTY,
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
