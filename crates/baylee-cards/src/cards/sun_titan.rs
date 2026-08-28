//! Sun Titan — {4}{W}{W} — Creature — Giant
//! Oracle: Vigilance
//! Oracle: Whenever this creature enters or attacks, you may return target permanent card with mana value 3 or less from your graveyard to the battlefield.
//! Set: SOC #178 — Secrets of Strixhaven Commander | Scryfall ID: 3d6eacf2-f6c7-4ede-b5a5-7463602699ae | Oracle ID: b2e950fb-cb7e-40a0-a311-5bbdd0477b29
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(158),
    oracle_id: "b2e950fb-cb7e-40a0-a311-5bbdd0477b29",
    scryfall_id: "3d6eacf2-f6c7-4ede-b5a5-7463602699ae",
    faces: &[FaceDef {
        name: "Sun Titan",
        mana_cost: baylee_core::mana!("{4}{W}{W}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[subtypes::creature::GIANT],
        power: Some(6),
        toughness: Some(6),
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
