//! Crib Swap — {2}{W} — Kindred Instant — Shapeshifter
//! Oracle: Changeling (This card is every creature type.)
//! Oracle: Exile target creature. Its controller creates a 1/1 colorless Shapeshifter creature token with changeling.
//! Set: ECL #11 — Lorwyn Eclipsed | Scryfall ID: 8f2fb3c6-af75-47a3-9f97-521872c32890 | Oracle ID: 2987c385-011a-4032-a516-a46d1e9dc9e8
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(26),
    oracle_id: "2987c385-011a-4032-a516-a46d1e9dc9e8",
    scryfall_id: "8f2fb3c6-af75-47a3-9f97-521872c32890",
    faces: &[FaceDef {
        name: "Crib Swap",
        mana_cost: baylee_core::mana!("{2}{W}"),
        types: TypeSet::KINDRED.union(TypeSet::INSTANT),
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[subtypes::creature::SHAPESHIFTER],
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
