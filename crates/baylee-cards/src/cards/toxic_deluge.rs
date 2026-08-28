//! Toxic Deluge — {2}{B} — Sorcery
//! Oracle: As an additional cost to cast this spell, pay X life.
//! Oracle: All creatures get -X/-X until end of turn.
//! Set: MSC #161 — Marvel Super Heroes Commander | Scryfall ID: de5afccc-8d42-4bd6-b068-b9ea2361655e | Oracle ID: afaef788-34d1-460b-b884-9d7ae6ddeb18
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(172),
    oracle_id: "afaef788-34d1-460b-b884-9d7ae6ddeb18",
    scryfall_id: "de5afccc-8d42-4bd6-b068-b9ea2361655e",
    faces: &[FaceDef {
        name: "Toxic Deluge",
        mana_cost: baylee_core::mana!("{2}{B}"),
        types: TypeSet::SORCERY,
        supertypes: SupertypeSet::EMPTY,
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
