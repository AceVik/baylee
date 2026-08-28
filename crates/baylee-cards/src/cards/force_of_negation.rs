//! Force of Negation — {1}{U}{U} — Instant
//! Oracle: If it's not your turn, you may exile a blue card from your hand rather than pay this spell's mana cost.
//! Oracle: Counter target noncreature spell. If that spell is countered this way, exile it instead of putting it into its owner's graveyard.
//! Set: 2X2 #50 — Double Masters 2022 | Scryfall ID: 1825a719-1b2a-4af9-9cd2-7cb497cd0317 | Oracle ID: ac2173f9-f223-440a-9231-fd98762bdc6f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(53),
    oracle_id: "ac2173f9-f223-440a-9231-fd98762bdc6f",
    scryfall_id: "1825a719-1b2a-4af9-9cd2-7cb497cd0317",
    faces: &[FaceDef {
        name: "Force of Negation",
        mana_cost: baylee_core::mana!("{1}{U}{U}"),
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
