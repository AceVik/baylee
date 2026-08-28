//! Mirrorhall Mimic // Ghastly Mimicry — (no cost) — Creature — Spirit // Enchantment — Aura
//! Set: VOW #68 — Innistrad: Crimson Vow | Scryfall ID: 823ad188-bd56-476d-9853-bed90bfad582 | Oracle ID: 5768fe50-a134-492c-a725-5ed02610c39f
//! Face: Mirrorhall Mimic — {3}{U} — Creature — Spirit
//! Face: Ghastly Mimicry —  — Enchantment — Aura
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(95),
    oracle_id: "5768fe50-a134-492c-a725-5ed02610c39f",
    scryfall_id: "823ad188-bd56-476d-9853-bed90bfad582",
    faces: &[
        FaceDef {
            name: "Mirrorhall Mimic",
            mana_cost: baylee_core::mana!("{3}{U}"),
            types: TypeSet::CREATURE,
            supertypes: SupertypeSet::EMPTY,
            subtypes: &[subtypes::creature::SPIRIT],
            power: Some(0),
            toughness: Some(0),
            loyalty: None,
        },
        FaceDef {
            name: "Ghastly Mimicry",
            mana_cost: ManaCost::ZERO,
            types: TypeSet::ENCHANTMENT,
            supertypes: SupertypeSet::EMPTY,
            subtypes: &[subtypes::enchantment::AURA],
            power: None,
            toughness: None,
            loyalty: None,
        },
    ],
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
