//! Doubling Season — {4}{G} — Enchantment
//! Oracle: If an effect would create one or more tokens under your control, it creates twice that many of those tokens instead.
//! Oracle: If an effect would put one or more counters on a permanent you control, it puts twice that many of those counters on that permanent instead.
//! Set: FDN #216 — Foundations | Scryfall ID: f2c4f80e-84a0-463b-82c3-5c6503809351 | Oracle ID: 01546b7d-a233-4176-8843-d732074dc5b6
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(35),
    oracle_id: "01546b7d-a233-4176-8843-d732074dc5b6",
    scryfall_id: "f2c4f80e-84a0-463b-82c3-5c6503809351",
    faces: &[FaceDef {
        name: "Doubling Season",
        mana_cost: baylee_core::mana!("{4}{G}"),
        types: TypeSet::ENCHANTMENT,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        power: None,
        toughness: None,
        loyalty: None,
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
