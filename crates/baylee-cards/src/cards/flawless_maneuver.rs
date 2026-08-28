//! Flawless Maneuver — {2}{W} — Instant
//! Oracle: If you control a commander, you may cast this spell without paying its mana cost.
//! Oracle: Creatures you control gain indestructible until end of turn.
//! Set: CMM #24 — Commander Masters | Scryfall ID: ab12f69e-1491-47a8-8c46-d85bbf637ff6 | Oracle ID: 4e183439-17d2-47ff-9d99-5e22821d91e3
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(51),
    oracle_id: "4e183439-17d2-47ff-9d99-5e22821d91e3",
    scryfall_id: "ab12f69e-1491-47a8-8c46-d85bbf637ff6",
    faces: &[FaceDef {
        name: "Flawless Maneuver",
        mana_cost: baylee_core::mana!("{2}{W}"),
        types: TypeSet::INSTANT,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        power: None,
        toughness: None,
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
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
