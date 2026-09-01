//! Spellseeker — {2}{U} — Creature — Human Wizard
//! Oracle: When this creature enters, you may search your library for an instant or sorcery card with mana value 2 or less, reveal it, put it into your hand, then shuffle.
//! Set: CMM #120 — Commander Masters | Scryfall ID: a749c591-2fbe-41d8-ac5b-56ebce82d33e | Oracle ID: 47a785ed-8095-4685-8daa-02c4e2b0ffcd
// IMPLEMENTED — ETB optional tutor for cheap instants/sorceries (reveal M3).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet, PartnerKind,
    SearchDest, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static CHEAP_SPELL: Filter = Filter::And(&[
    Filter::Or(&[
        Filter::HasType(TypeSet::INSTANT),
        Filter::HasType(TypeSet::SORCERY),
    ]),
    Filter::CmcAtMost(2),
]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(155),
    oracle_id: "47a785ed-8095-4685-8daa-02c4e2b0ffcd",
    scryfall_id: "a749c591-2fbe-41d8-ac5b-56ebce82d33e",
    faces: &[FaceDef {
        name: "Spellseeker",
        mana_cost: baylee_core::mana!("{2}{U}"),
        types: TypeSet::CREATURE,
        subtypes: &[creature::HUMAN, creature::WIZARD],
        power: Some(1),
        toughness: Some(1),
        ..FaceDef::DEFAULT
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Triggered {
        trigger: Trigger::EntersBattlefield(&Filter::This),
        once_per_turn: false,
        effects: &[Effect::SearchLibrary {
            filter: &CHEAP_SPELL,
            dest: SearchDest::Hand,
            tapped: false,
            shuffle: true,
            optional: true,
        }],
        targets: None,
    }],
    ..CardDef::DEFAULT
};
