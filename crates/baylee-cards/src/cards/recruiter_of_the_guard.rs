//! Recruiter of the Guard — {2}{W} — Creature — Human Soldier
//! Oracle: When this creature enters, you may search your library for a creature card with toughness 2 or less, reveal it, put it into your hand, then shuffle.
//! Set: MH3 #266 — Modern Horizons 3 | Scryfall ID: 8e4c6ba1-1abc-478f-9b7c-97e9e3c92fb0 | Oracle ID: d521a329-a53a-4962-810a-2abed80df260
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(126),
    oracle_id: "d521a329-a53a-4962-810a-2abed80df260",
    scryfall_id: "8e4c6ba1-1abc-478f-9b7c-97e9e3c92fb0",
    faces: &[FaceDef {
        name: "Recruiter of the Guard",
        mana_cost: baylee_core::mana!("{2}{W}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[subtypes::creature::HUMAN, subtypes::creature::SOLDIER],
        power: Some(1),
        toughness: Some(1),
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
