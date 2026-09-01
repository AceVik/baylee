//! Demonic Tutor — {1}{B} — Sorcery
//! Oracle: Search your library for a card, put that card into your hand, then shuffle.
//! Set: CMM #150 — Commander Masters | Scryfall ID: a24b4cb6-cebb-428b-8654-74347a6a8d63 | Oracle ID: 82004860-e589-4e38-8d61-8c0210e4ea39
// IMPLEMENTED — unrestricted tutor to hand.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, Find, KeywordSet,
    PartnerKind, SearchDest,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static ANY_CARD: Filter = Filter::Any;

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(32),
    oracle_id: "82004860-e589-4e38-8d61-8c0210e4ea39",
    scryfall_id: "a24b4cb6-cebb-428b-8654-74347a6a8d63",
    faces: &[FaceDef {
        name: "Demonic Tutor",
        mana_cost: baylee_core::mana!("{1}{B}"),
        types: TypeSet::SORCERY,
        ..FaceDef::DEFAULT
    }],
    color_identity: ColorSet::from_slice(&[Color::Black]),
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Spell {
        effects: &[Effect::SearchLibrary {
            filter: &ANY_CARD,
            finds: &[Find::HAND],
            shuffle: true,
            optional: false,
        }],
        targets: None,
    }],
    ..CardDef::DEFAULT
};

// Engine-level coverage via s4 scenario tests: tutoring puts any chosen
// library card into hand and shuffles.
