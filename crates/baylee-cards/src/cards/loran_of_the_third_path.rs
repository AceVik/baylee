//! Loran of the Third Path — {2}{W} — Legendary Creature — Human Artificer
//! Oracle: Vigilance
//! Oracle: When Loran enters, destroy up to one target artifact or enchantment.
//! Oracle: {T}: You and target opponent each draw a card.
//! Set: MKC #71 — Murders at Karlov Manor Commander | Scryfall ID: 9e83a0ef-4fea-45ba-86c0-130d6687f7fe | Oracle ID: b3d81980-76f2-44e2-b1c9-01e30c726312
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(87),
    oracle_id: "b3d81980-76f2-44e2-b1c9-01e30c726312",
    scryfall_id: "9e83a0ef-4fea-45ba-86c0-130d6687f7fe",
    faces: &[FaceDef {
        name: "Loran of the Third Path",
        mana_cost: baylee_core::mana!("{2}{W}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[subtypes::creature::HUMAN, subtypes::creature::ARTIFICER],
        power: Some(2),
        toughness: Some(1),
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::Legendary,
    partner: PartnerKind::None,
    coverage: Coverage::Unimplemented,
    abilities: &[],
};

#[cfg(test)]
mod tests {
    // TODO(card): implement abilities + tests, see docs/card-dsl.md.
}
