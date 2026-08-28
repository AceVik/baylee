//! Rite of Replication — {2}{U}{U} — Sorcery
//! Oracle: Kicker {5} (You may pay an additional {5} as you cast this spell.)
//! Oracle: Create a token that's a copy of target creature. If this spell was kicked, create five of those tokens instead.
//! Set: SOC #202 — Secrets of Strixhaven Commander | Scryfall ID: 5032d71d-d9f8-498c-97d1-271c2e9c1c47 | Oracle ID: fb60739e-1dc3-481d-a056-ad72e665c680
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(135),
    oracle_id: "fb60739e-1dc3-481d-a056-ad72e665c680",
    scryfall_id: "5032d71d-d9f8-498c-97d1-271c2e9c1c47",
    faces: &[FaceDef {
        name: "Rite of Replication",
        mana_cost: baylee_core::mana!("{2}{U}{U}"),
        types: TypeSet::SORCERY,
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
