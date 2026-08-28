//! Cyclonic Rift — {1}{U} — Instant
//! Oracle: Return target nonland permanent you don't control to its owner's hand.
//! Oracle: Overload {6}{U} (You may cast this spell for its overload cost. If you do, change "target" in its text to "each.")
//! Set: RVR #40 — Ravnica Remastered | Scryfall ID: dfb7c4b9-f2f4-4d4e-baf2-86551c8150fe | Oracle ID: d75b9c82-1b49-4c3e-a1b5-aeef57d6644b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(29),
    oracle_id: "d75b9c82-1b49-4c3e-a1b5-aeef57d6644b",
    scryfall_id: "dfb7c4b9-f2f4-4d4e-baf2-86551c8150fe",
    faces: &[FaceDef {
        name: "Cyclonic Rift",
        mana_cost: baylee_core::mana!("{1}{U}"),
        types: TypeSet::INSTANT,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        power: None,
        toughness: None,
        loyalty: None,
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
