//! Vendilion Clique — {1}{U}{U} — Legendary Creature — Faerie Wizard
//! Oracle: Flash
//! Oracle: Flying
//! Oracle: When Vendilion Clique enters, look at target player's hand. You may choose a nonland card from it. If you do, that player reveals the chosen card, puts it on the bottom of their library, then draws a card.
//! Set: A25 #76 — Masters 25 | Scryfall ID: cd702cf1-10ca-4448-9fb1-b6de635e839c | Oracle ID: 244d4807-0802-41bc-9460-55ac38a28a72
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(181),
    oracle_id: "244d4807-0802-41bc-9460-55ac38a28a72",
    scryfall_id: "cd702cf1-10ca-4448-9fb1-b6de635e839c",
    faces: &[FaceDef {
        name: "Vendilion Clique",
        mana_cost: baylee_core::mana!("{1}{U}{U}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[subtypes::creature::FAERIE, subtypes::creature::WIZARD],
        power: Some(3),
        toughness: Some(1),
        loyalty: None,
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
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
