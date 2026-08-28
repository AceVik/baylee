//! Machine God's Effigy — {4} — Artifact
//! Oracle: You may have this artifact enter as a copy of any creature on the battlefield, except it's an artifact and it has "{T}: Add {U}." (It's not a creature.)
//! Oracle: {T}: Add {U}.
//! Set: BRC #16 — The Brothers' War Commander | Scryfall ID: 637f69c2-ba24-42d1-9345-8ebdb04b6904 | Oracle ID: 64ebdd6f-acde-4aab-a86b-2798bad5f70c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(89),
    oracle_id: "64ebdd6f-acde-4aab-a86b-2798bad5f70c",
    scryfall_id: "637f69c2-ba24-42d1-9345-8ebdb04b6904",
    faces: &[FaceDef {
        name: "Machine God's Effigy",
        mana_cost: baylee_core::mana!("{4}"),
        types: TypeSet::ARTIFACT,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        power: None,
        toughness: None,
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
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
