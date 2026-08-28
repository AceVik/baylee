//! Harabaz Druid — {1}{G} — Creature — Human Druid Ally
//! Oracle: {T}: Add X mana of any one color, where X is the number of Allies you control.
//! Set: WWK #105 — Worldwake | Scryfall ID: 78a538cf-2291-49aa-8429-17d97d454479 | Oracle ID: ead985ec-f29f-4a3b-b8b1-061142cc5bd1
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(66),
    oracle_id: "ead985ec-f29f-4a3b-b8b1-061142cc5bd1",
    scryfall_id: "78a538cf-2291-49aa-8429-17d97d454479",
    faces: &[FaceDef {
        name: "Harabaz Druid",
        mana_cost: baylee_core::mana!("{1}{G}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[
            subtypes::creature::HUMAN,
            subtypes::creature::DRUID,
            subtypes::creature::ALLY,
        ],
        power: Some(0),
        toughness: Some(1),
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
    }],
    color_identity: ColorSet::from_slice(&[Color::Green]),
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
