//! Emeritus of Woe // Demonic Tutor — {3}{B} // {1}{B} — Creature — Vampire Warlock // Sorcery
//! Set: SOS #80 — Secrets of Strixhaven | Scryfall ID: 7eb9e83d-515d-4911-a06b-9982200277b2 | Oracle ID: 93056597-b964-421f-be2f-e92abef1c2a4
//! Face: Emeritus of Woe — {3}{B} — Creature — Vampire Warlock
//! Face: Demonic Tutor — {1}{B} — Sorcery
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(41),
    oracle_id: "93056597-b964-421f-be2f-e92abef1c2a4",
    scryfall_id: "7eb9e83d-515d-4911-a06b-9982200277b2",
    faces: &[
        FaceDef {
            name: "Emeritus of Woe",
            mana_cost: baylee_core::mana!("{3}{B}"),
            types: TypeSet::CREATURE,
            supertypes: SupertypeSet::EMPTY,
            subtypes: &[subtypes::creature::VAMPIRE, subtypes::creature::WARLOCK],
            power: Some(5),
            toughness: Some(4),
            loyalty: None,
        },
        FaceDef {
            name: "Demonic Tutor",
            mana_cost: baylee_core::mana!("{1}{B}"),
            types: TypeSet::SORCERY,
            supertypes: SupertypeSet::EMPTY,
            subtypes: &[],
            power: None,
            toughness: None,
            loyalty: None,
        },
    ],
    color_identity: ColorSet::from_slice(&[Color::Black]),
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
