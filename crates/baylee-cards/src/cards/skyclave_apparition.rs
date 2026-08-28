//! Skyclave Apparition — {1}{W}{W} — Creature — Kor Spirit
//! Oracle: When this creature enters, exile up to one target nonland, nontoken permanent you don't control with mana value 4 or less.
//! Oracle: When this creature leaves the battlefield, the exiled card's owner creates an X/X blue Illusion creature token, where X is the mana value of the exiled card.
//! Set: SOC #173 — Secrets of Strixhaven Commander | Scryfall ID: e671de25-c47c-48a1-919b-6aa30dab142f | Oracle ID: d90af00a-d322-4265-9954-7b1e80702e18
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(146),
    oracle_id: "d90af00a-d322-4265-9954-7b1e80702e18",
    scryfall_id: "e671de25-c47c-48a1-919b-6aa30dab142f",
    faces: &[FaceDef {
        name: "Skyclave Apparition",
        mana_cost: baylee_core::mana!("{1}{W}{W}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[subtypes::creature::KOR, subtypes::creature::SPIRIT],
        power: Some(2),
        toughness: Some(2),
        loyalty: None,
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
