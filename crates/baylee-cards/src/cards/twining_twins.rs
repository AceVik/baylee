//! Twining Twins // Swift Spiral — {2}{U}{U} // {1}{W} — Creature — Faerie Wizard // Instant — Adventure
//! Set: WOE #240 — Wilds of Eldraine | Scryfall ID: 043718ea-59f6-4d1a-94c5-271704c1a38a | Oracle ID: 105aea98-8eb9-4fb2-a0cb-7c7513317c5b
//! Face: Twining Twins — {2}{U}{U} — Creature — Faerie Wizard
//! Face: Swift Spiral — {1}{W} — Instant — Adventure
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(176),
    oracle_id: "105aea98-8eb9-4fb2-a0cb-7c7513317c5b",
    scryfall_id: "043718ea-59f6-4d1a-94c5-271704c1a38a",
    faces: &[
        FaceDef {
            name: "Twining Twins",
            mana_cost: baylee_core::mana!("{2}{U}{U}"),
            types: TypeSet::CREATURE,
            supertypes: SupertypeSet::EMPTY,
            subtypes: &[subtypes::creature::FAERIE, subtypes::creature::WIZARD],
            power: Some(4),
            toughness: Some(4),
            loyalty: None,
            alternative_costs: &[],
            additional_costs: &[],
            mandatory_additional_costs: &[],
        },
        FaceDef {
            name: "Swift Spiral",
            mana_cost: baylee_core::mana!("{1}{W}"),
            types: TypeSet::INSTANT,
            supertypes: SupertypeSet::EMPTY,
            subtypes: &[subtypes::spell::ADVENTURE],
            power: None,
            toughness: None,
            loyalty: None,
            alternative_costs: &[],
            additional_costs: &[],
            mandatory_additional_costs: &[],
        },
    ],
    color_identity: ColorSet::from_slice(&[Color::Blue, Color::White]),
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
