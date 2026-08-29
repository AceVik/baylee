//! Everybody Lives! — {1}{W} — Instant
//! Oracle: All creatures gain hexproof and indestructible until end of turn. Players gain hexproof until end of turn. Players can't lose life this turn and players can't lose the game or win the game this turn.
//! Set: WHO #18 — Doctor Who | Scryfall ID: 9dab0052-7f0c-4b56-847f-20552666a271 | Oracle ID: 39213de3-6a4a-4879-a7f9-70f45013765e
// IMPLEMENTED — creature hexproof+indestructible EOT, no-life-loss,
// no-lose/no-win suppression, and player hexproof (ChoosePlayer filters
// hexproofed players out).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, Duration, Effect, FaceDef, Filter, KeywordSet,
    Layer, Modifier, PartnerKind,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static ALL_CREATURES: Filter = Filter::HasType(TypeSet::CREATURE);
static HEXPROOF_INDESTRUCTIBLE: KeywordSet = KeywordSet::HEXPROOF.union(KeywordSet::INDESTRUCTIBLE);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(47),
    oracle_id: "39213de3-6a4a-4879-a7f9-70f45013765e",
    scryfall_id: "9dab0052-7f0c-4b56-847f-20552666a271",
    faces: &[FaceDef {
        name: "Everybody Lives!",
        mana_cost: baylee_core::mana!("{1}{W}"),
        types: TypeSet::INSTANT,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        power: None,
        toughness: None,
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
        adventure: false,
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Spell {
        effects: &[
            Effect::CreateContinuousEffect {
                layer: Layer::Ability,
                filter: &ALL_CREATURES,
                modifier: Modifier::AddKeyword(HEXPROOF_INDESTRUCTIBLE),
                duration: Duration::UntilEndOfTurn,
            },
            Effect::CreateContinuousEffect {
                layer: Layer::Text,
                filter: &Filter::Any,
                modifier: Modifier::CantLoseLife,
                duration: Duration::UntilEndOfTurn,
            },
            Effect::CreateContinuousEffect {
                layer: Layer::Text,
                filter: &Filter::Any,
                modifier: Modifier::PlayersCantLose,
                duration: Duration::UntilEndOfTurn,
            },
            Effect::CreateContinuousEffect {
                layer: Layer::Text,
                filter: &Filter::Any,
                modifier: Modifier::PlayerHexproof,
                duration: Duration::UntilEndOfTurn,
            },
        ],
        targets: None,
    }],
};

#[cfg(test)]
mod tests {}
