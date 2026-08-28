//! Restoration Angel — {3}{W} — Creature — Angel
//! Oracle: Flash
//! Oracle: Flying
//! Oracle: When this creature enters, you may exile target non-Angel creature you control, then return that card to the battlefield under your control.
//! Set: INR #38 — Innistrad Remastered | Scryfall ID: f17f85d3-58e5-4128-90c5-98b524256af8 | Oracle ID: dfbd3afc-9905-4cff-a4f4-df08a4d0a7fa
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(131),
    oracle_id: "dfbd3afc-9905-4cff-a4f4-df08a4d0a7fa",
    scryfall_id: "f17f85d3-58e5-4128-90c5-98b524256af8",
    faces: &[FaceDef {
        name: "Restoration Angel",
        mana_cost: baylee_core::mana!("{3}{W}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[subtypes::creature::ANGEL],
        power: Some(3),
        toughness: Some(4),
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
