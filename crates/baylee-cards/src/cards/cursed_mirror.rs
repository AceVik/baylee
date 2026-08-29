//! Cursed Mirror — {2}{R} — Artifact
//! Oracle: {T}: Add {R}.
//! Oracle: As this artifact enters, you may have it become a copy of any creature on the battlefield until end of turn, except it has haste.
//! Set: SOC #242 — Secrets of Strixhaven Commander | Scryfall ID: 077392b3-6b06-46c8-8737-51e85f690448 | Oracle ID: 4d67e2a7-4aa7-44cc-853b-500d7aac046d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(28),
    oracle_id: "4d67e2a7-4aa7-44cc-853b-500d7aac046d",
    scryfall_id: "077392b3-6b06-46c8-8737-51e85f690448",
    faces: &[FaceDef {
        name: "Cursed Mirror",
        mana_cost: baylee_core::mana!("{2}{R}"),
        types: TypeSet::ARTIFACT,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        power: None,
        toughness: None,
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
        abilities: &[],
        castable_from_hand: true,
    }],
    color_identity: ColorSet::from_slice(&[Color::Red]),
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
