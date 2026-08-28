//! Aminatou, the Fateshifter — {W}{U}{B} — Legendary Planeswalker — Aminatou
//! Oracle: +1: Draw a card, then put a card from your hand on top of your library.
//! Oracle: −1: Exile another target permanent you own, then return it to the battlefield under your control.
//! Oracle: −6: Choose left or right. Each player gains control of all nonland permanents other than Aminatou controlled by the next player in the chosen direction.
//! Oracle: Aminatou, the Fateshifter can be your commander.
//! Set: 2X2 #169 — Double Masters 2022 | Scryfall ID: bc010302-e715-4946-89eb-a214e0b836ba | Oracle ID: 3a30089d-cd2d-49be-9b06-7a2454117692
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(3),
    oracle_id: "3a30089d-cd2d-49be-9b06-7a2454117692",
    scryfall_id: "bc010302-e715-4946-89eb-a214e0b836ba",
    faces: &[FaceDef {
        name: "Aminatou, the Fateshifter",
        mana_cost: baylee_core::mana!("{W}{U}{B}"),
        types: TypeSet::PLANESWALKER,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[subtypes::planeswalker::AMINATOU],
        power: None,
        toughness: None,
        loyalty: Some(3),
    }],
    color_identity: ColorSet::from_slice(&[Color::Black, Color::Blue, Color::White]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::ExplicitlyAllowed,
    partner: PartnerKind::None,
    coverage: Coverage::Unimplemented,
    abilities: &[],
};

#[cfg(test)]
mod tests {
    // TODO(card): implement abilities + tests, see docs/card-dsl.md.
}
