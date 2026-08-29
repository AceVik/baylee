//! Elspeth, Storm Slayer — {3}{W}{W} — Legendary Planeswalker — Elspeth
//! Oracle: If one or more tokens would be created under your control, twice that many of those tokens are created instead.
//! Oracle: +1: Create a 1/1 white Soldier creature token.
//! Oracle: 0: Put a +1/+1 counter on each creature you control. Those creatures gain flying until your next turn.
//! Oracle: −3: Destroy target creature an opponent controls with mana value 3 or greater.
//! Set: TDM #11 — Tarkir: Dragonstorm | Scryfall ID: 73a065e3-b530-4e62-ab3c-4f6f908184ec | Oracle ID: f78af825-023a-42e9-8374-5c52303a1417
// PARTIAL — token doubling, +1 token, −3 destroy implemented; 0's flying-
// until-next-turn needs UntilYourNextTurn duration (M2+).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, Amount, CardDef, CommanderRule, CounterKind, Coverage, Duration, Effect, FaceDef,
    Filter, KeywordSet, Layer, Modifier, PartnerKind, ReplacementRule, TargetReq, TargetSpec,
    TokenDef,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature, planeswalker};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static YOURS: Filter = Filter::ControlledByYou;
static BIG_ENEMY_CREATURE: Filter = Filter::And(&[
    Filter::ControlledByOpponent,
    Filter::HasType(TypeSet::CREATURE),
    Filter::CmcAtLeast(3),
]);
static YOUR_CREATURES: Filter =
    Filter::And(&[Filter::ControlledByYou, Filter::HasType(TypeSet::CREATURE)]);

static SOLDIER: TokenDef = TokenDef {
    name: "Soldier",
    colors: ColorSet::from_slice(&[Color::White]),
    types: TypeSet::CREATURE,
    supertypes: SupertypeSet::EMPTY,
    subtypes: &[creature::SOLDIER],
    power: Some(1),
    toughness: Some(1),
    keywords: KeywordSet::EMPTY,
};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(40),
    oracle_id: "f78af825-023a-42e9-8374-5c52303a1417",
    scryfall_id: "73a065e3-b530-4e62-ab3c-4f6f908184ec",
    faces: &[FaceDef {
        name: "Elspeth, Storm Slayer",
        mana_cost: baylee_core::mana!("{3}{W}{W}"),
        types: TypeSet::PLANESWALKER,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[planeswalker::ELSPETH],
        power: None,
        toughness: None,
        loyalty: Some(5),
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
        adventure: false,
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::Legendary,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[
        AbilityDef::Replacement(ReplacementRule::DoubleTokenCreation {
            controller_filter: &YOURS,
        }),
        AbilityDef::Loyalty {
            cost: 1,
            effects: &[Effect::CreateToken { token: &SOLDIER }],
            target: None,
        },
        AbilityDef::Loyalty {
            cost: 0,
            effects: &[
                Effect::AddCounterFilter {
                    filter: &YOUR_CREATURES,
                    kind: CounterKind::P1P1,
                    amount: Amount::Fixed(1),
                },
                Effect::CreateContinuousEffect {
                    layer: Layer::Ability,
                    filter: &YOUR_CREATURES,
                    modifier: Modifier::AddKeyword(KeywordSet::FLYING),
                    duration: Duration::UntilYourNextTurn,
                },
            ],
            target: None,
        },
        AbilityDef::Loyalty {
            cost: -3,
            effects: &[Effect::Destroy {
                target: TargetSpec::Object(&BIG_ENEMY_CREATURE),
            }],
            target: Some(TargetSpec::Object(&BIG_ENEMY_CREATURE)),
        },
    ],
};

#[cfg(test)]
mod tests {}
