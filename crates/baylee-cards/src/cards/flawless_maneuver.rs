//! Flawless Maneuver — {2}{W} — Instant
//! Oracle: If you control a commander, you may cast this spell without paying its mana cost.
//! Oracle: Creatures you control gain indestructible until end of turn.
//! Set: CMM #24 — Commander Masters | Scryfall ID: ab12f69e-1491-47a8-8c46-d85bbf637ff6 | Oracle ID: 4e183439-17d2-47ff-9d99-5e22821d91e3
// IMPLEMENTED — commander-conditional free cast + team indestructible.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, AltCondition, AlternativeCost, CardDef, CommanderRule, Cost, Coverage, Duration,
    Effect, FaceDef, Filter, KeywordSet, Layer, Modifier, PartnerKind,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static YOUR_CREATURES: Filter =
    Filter::And(&[Filter::HasType(TypeSet::CREATURE), Filter::ControlledByYou]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(51),
    oracle_id: "4e183439-17d2-47ff-9d99-5e22821d91e3",
    scryfall_id: "ab12f69e-1491-47a8-8c46-d85bbf637ff6",
    faces: &[FaceDef {
        name: "Flawless Maneuver",
        mana_cost: baylee_core::mana!("{2}{W}"),
        types: TypeSet::INSTANT,
        alternative_costs: &[AlternativeCost {
            cost: Cost::FREE,
            condition: AltCondition::CommanderControlled,
        }],
        ..FaceDef::DEFAULT
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Spell {
        effects: &[Effect::CreateContinuousEffect {
            layer: Layer::Ability,
            filter: &YOUR_CREATURES,
            modifier: Modifier::AddKeyword(KeywordSet::INDESTRUCTIBLE),
            duration: Duration::UntilEndOfTurn,
        }],
        targets: None,
    }],
    ..CardDef::DEFAULT
};
