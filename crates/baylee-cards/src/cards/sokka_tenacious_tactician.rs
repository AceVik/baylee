//! Sokka, Tenacious Tactician — {1}{U}{R} — Legendary Creature — Human Warrior Ally
//! Oracle: Menace, prowess (Whenever you cast a noncreature spell, this creature gets +1/+1 until end of turn.)
//! Oracle: Other Allies you control have menace and prowess.
//! Oracle: Whenever you cast a noncreature spell, create a 1/1 white Ally creature token.
//! Set: TLA #241 — Avatar: The Last Airbender | Scryfall ID: f0fa5897-1da7-488f-bb19-1632e969c050 | Oracle ID: 6b68acc2-b9d5-495b-8054-c04bae1349f1
// IMPLEMENTED — menace + prowess (engine-level keyword trigger) + Ally
// grants (layer 6) + token on noncreature spells.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, Amount, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet,
    Layer, Modifier, PartnerKind, StaticAbility, TokenDef, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static YOUR_ALLIES: Filter = Filter::And(&[
    Filter::ControlledByYou,
    Filter::HasSubtype(creature::ALLY),
    Filter::Another,
]);
static NONCREATURE_SPELL: Filter = Filter::And(&[
    Filter::ControlledByYou,
    Filter::LacksType(TypeSet::CREATURE),
]);

static ALLY_TOKEN: TokenDef = TokenDef {
    name: "Ally",
    colors: ColorSet::from_slice(&[Color::White]),
    types: TypeSet::CREATURE,
    supertypes: SupertypeSet::EMPTY,
    subtypes: &[creature::ALLY],
    power: Some(1),
    toughness: Some(1),
    keywords: KeywordSet::EMPTY,
};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(149),
    oracle_id: "6b68acc2-b9d5-495b-8054-c04bae1349f1",
    scryfall_id: "f0fa5897-1da7-488f-bb19-1632e969c050",
    faces: &[FaceDef {
        name: "Sokka, Tenacious Tactician",
        mana_cost: baylee_core::mana!("{1}{U}{R}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[creature::HUMAN, creature::WARRIOR, creature::ALLY],
        power: Some(2),
        toughness: Some(2),
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
        abilities: &[],
        castable_from_hand: true,
        miracle: None,
        delve: false,
        convoke: false,
        cost_reduction: None,
        disturb: false,
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue, Color::Red]),
    keywords: KeywordSet::MENACE.union(KeywordSet::PROWESS),
    commander: CommanderRule::Legendary,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[
        AbilityDef::Static(StaticAbility {
            layer: Layer::Ability,
            filter: YOUR_ALLIES,
            modifier: Modifier::AddKeyword(KeywordSet::MENACE.union(KeywordSet::PROWESS)),
            cross_zone: false,
        }),
        AbilityDef::Triggered {
            trigger: Trigger::SpellCast(&NONCREATURE_SPELL),
            once_per_turn: false,
            effects: &[Effect::CreateTokenN {
                token: &ALLY_TOKEN,
                amount: Amount::Fixed(1),
            }],
            targets: None,
        },
    ],
};

#[cfg(test)]
mod tests {}
