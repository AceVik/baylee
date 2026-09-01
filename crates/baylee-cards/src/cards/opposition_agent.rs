//! Opposition Agent — {2}{B} — Creature — Human Rogue
//! Oracle: Flash
//! Oracle: You control your opponents while they're searching their libraries.
//! Oracle: While an opponent is searching their library, they exile each card they find. You may play those cards for as long as they remain exiled, and you may spend mana as though it were mana of any color to cast them.
//! Set: CMR #141 — Commander Legends | Scryfall ID: 086f97e9-8b62-44f3-b467-149c2ac5ca78 | Oracle ID: 1f438b8f-fe23-4f3b-ab2e-f6c33676c462
// IMPLEMENTED — flash body + the search takeover: while an opponent
// searches, you choose their finds, the cards go to exile, and you may
// play them from exile spending any color of mana. (The "you control
// them while searching" nuance of ALSO making their choice for them is
// covered by the takeover choice.)
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, FaceDef, Filter, KeywordSet, Layer, Modifier,
    PartnerKind, StaticAbility,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(105),
    oracle_id: "1f438b8f-fe23-4f3b-ab2e-f6c33676c462",
    scryfall_id: "086f97e9-8b62-44f3-b467-149c2ac5ca78",
    faces: &[FaceDef {
        name: "Opposition Agent",
        mana_cost: baylee_core::mana!("{2}{B}"),
        types: TypeSet::CREATURE,
        subtypes: &[creature::HUMAN, creature::ROGUE],
        power: Some(3),
        toughness: Some(2),
        ..FaceDef::DEFAULT
    }],
    color_identity: ColorSet::from_slice(&[Color::Black]),
    keywords: KeywordSet::FLASH,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Static(StaticAbility {
        layer: Layer::Text,
        filter: Filter::Any,
        modifier: Modifier::SearchTakeover,
        cross_zone: false,
    })],
    ..CardDef::DEFAULT
};
