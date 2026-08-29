//! Sakashima of a Thousand Faces — {3}{U} — Legendary Creature — Human Rogue
//! Oracle: You may have Sakashima enter as a copy of another creature you control, except it has Sakashima's other abilities.
//! Oracle: The "legend rule" doesn't apply to permanents you control.
//! Oracle: Partner (You can have two commanders if both have partner.)
//! Set: CMR #89 — Commander Legends | Scryfall ID: 714c3a1f-7b30-4ed8-8f38-6176758741fb | Oracle ID: 8ecdaf4b-4442-42da-9714-4257a83faf50
// IMPLEMENTED — clone of your creatures + legend rule suppression + partner.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, FaceDef, Filter, KeywordSet, Layer, Modifier,
    PartnerKind, StaticAbility, TargetSpec,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static YOUR_CREATURES: Filter = Filter::And(&[
    Filter::ControlledByYou,
    Filter::HasType(TypeSet::CREATURE),
    Filter::Another,
]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(137),
    oracle_id: "8ecdaf4b-4442-42da-9714-4257a83faf50",
    scryfall_id: "714c3a1f-7b30-4ed8-8f38-6176758741fb",
    faces: &[FaceDef {
        name: "Sakashima of a Thousand Faces",
        mana_cost: baylee_core::mana!("{3}{U}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[creature::HUMAN, creature::ROGUE],
        power: Some(3),
        toughness: Some(1),
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
        abilities: &[],
        castable_from_hand: true,
        miracle: None,
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::Legendary,
    partner: PartnerKind::Partner,
    coverage: Coverage::Implemented,
    abilities: &[
        AbilityDef::CopyOnEnter {
            target: TargetSpec::Object(&YOUR_CREATURES),
            mods: &[],
        },
        AbilityDef::Static(StaticAbility {
            layer: Layer::Text,
            filter: Filter::Any,
            modifier: Modifier::LegendRuleOff,
            cross_zone: false,
        }),
    ],
};

#[cfg(test)]
mod tests {}
