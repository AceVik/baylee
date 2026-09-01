//! Kodama's Reach — {2}{G} — Sorcery — Arcane
//! Oracle: Search your library for up to two basic land cards, reveal those cards, put one onto the battlefield tapped and the other into your hand, then shuffle.
//! Set: ECC #113 — Lorwyn Eclipsed Commander | Scryfall ID: 90c423cc-1264-4067-9c50-e7c88c68ef2d | Oracle ID: 1593ea18-2f2f-4ab4-83fb-6ccc0bec8a90
// IMPLEMENTED — the same two finds as Cultivate. Arcane is a spell subtype
// carrying no rules of its own; splice onto Arcane is not modelled and no
// card here has it.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, Find, KeywordSet,
    PartnerKind,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

/// A basic land card: the *supertype* Basic plus the land type (CR 205.4a).
static BASIC_LAND: Filter = Filter::And(&[
    Filter::HasSupertype(SupertypeSet::BASIC),
    Filter::HasType(TypeSet::LAND),
]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(195),
    oracle_id: "1593ea18-2f2f-4ab4-83fb-6ccc0bec8a90",
    scryfall_id: "90c423cc-1264-4067-9c50-e7c88c68ef2d",
    color_identity: ColorSet::from_slice(&[Color::Green]),
    faces: &[FaceDef {
        name: "Kodama's Reach",
        mana_cost: baylee_core::mana!("{2}{G}"),
        types: TypeSet::SORCERY,
        subtypes: &[subtypes::spell::ARCANE],
        ..FaceDef::DEFAULT
    }],
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Spell {
        effects: &[Effect::SearchLibrary {
            filter: &BASIC_LAND,
            finds: &[Find::BATTLEFIELD_TAPPED, Find::HAND],
            shuffle: true,
            optional: true, // "up to two"
        }],
        targets: None,
    }],
    ..CardDef::DEFAULT
};

// Engine-level coverage lives in baylee-engine (search_tests), on Cultivate:
// the same effect with the same two destinations.
