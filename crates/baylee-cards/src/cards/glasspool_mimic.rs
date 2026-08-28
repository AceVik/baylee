//! Glasspool Mimic // Glasspool Shore — (no cost) — Creature — Shapeshifter Rogue // Land
//! Set: ZNR #60 — Zendikar Rising | Scryfall ID: 5adcb500-8c77-4925-8e2c-1243502827d1 | Oracle ID: c178953c-3888-4edd-9d0c-265bd82b1d24
//! Face: Glasspool Mimic — {2}{U} — Creature — Shapeshifter Rogue
//! Face: Glasspool Shore —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(60),
    oracle_id: "c178953c-3888-4edd-9d0c-265bd82b1d24",
    scryfall_id: "5adcb500-8c77-4925-8e2c-1243502827d1",
    faces: &[
        FaceDef {
            name: "Glasspool Mimic",
            mana_cost: baylee_core::mana!("{2}{U}"),
            types: TypeSet::CREATURE,
            supertypes: SupertypeSet::EMPTY,
            subtypes: &[subtypes::creature::SHAPESHIFTER, subtypes::creature::ROGUE],
            power: Some(0),
            toughness: Some(0),
            loyalty: None,
            alternative_costs: &[],
            additional_costs: &[],
            mandatory_additional_costs: &[],
        enter_modifiers: &[],
        },
        FaceDef {
            name: "Glasspool Shore",
            mana_cost: ManaCost::ZERO,
            types: TypeSet::LAND,
            supertypes: SupertypeSet::EMPTY,
            subtypes: &[],
            power: None,
            toughness: None,
            loyalty: None,
            alternative_costs: &[],
            additional_costs: &[],
            mandatory_additional_costs: &[],
        enter_modifiers: &[],
        },
    ],
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
