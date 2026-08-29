//! Metamorphosis Fanatic — {4}{B}{B} — Creature — Human Cleric
//! Oracle: Lifelink
//! Oracle: When this creature enters, return up to one target creature card from your graveyard to the battlefield with a lifelink counter on it.
//! Oracle: Miracle {1}{B} (You may cast this card for its miracle cost when you draw it if it's the first card you drew this turn.)
//! Set: DSC #21 — Duskmourn: House of Horror Commander | Scryfall ID: 16448d95-ee21-4def-b880-26f6f159c213 | Oracle ID: 017aa9b3-a8ea-4588-9c50-e914a7d8e4ee
// IMPLEMENTED — lifelink 4/4 + ETB reanimate with a lifelink counter +
// miracle cast.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, Amount, CardDef, CommanderRule, CounterKind, Coverage, Effect, FaceDef, Filter,
    KeywordSet, PartnerKind, PlayerRel, TargetReq, TargetSpec, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static CREATURE_CARD: Filter = Filter::HasType(TypeSet::CREATURE);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(94),
    oracle_id: "017aa9b3-a8ea-4588-9c50-e914a7d8e4ee",
    scryfall_id: "16448d95-ee21-4def-b880-26f6f159c213",
    faces: &[FaceDef {
        name: "Metamorphosis Fanatic",
        mana_cost: baylee_core::mana!("{4}{B}{B}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[creature::HUMAN, creature::CLERIC],
        power: Some(4),
        toughness: Some(4),
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
        abilities: &[],
        castable_from_hand: true,
        miracle: Some(baylee_core::mana!("{1}{B}")),
    }],
    color_identity: ColorSet::from_slice(&[Color::Black]),
    keywords: KeywordSet::LIFELINK,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Triggered {
        trigger: Trigger::EntersBattlefield(&Filter::This),
        once_per_turn: false,
        effects: &[
            Effect::GraveyardToBattlefield {
                target: TargetSpec::CardInGraveyard(&CREATURE_CARD, PlayerRel::You),
            },
            Effect::AddCounter {
                kind: CounterKind::Lifelink,
                amount: Amount::Fixed(1),
            },
        ],
        targets: Some(TargetReq {
            spec: TargetSpec::CardInGraveyard(&CREATURE_CARD, PlayerRel::You),
            min: 0,
            max: 1,
            count_is_x: false,
        }),
    }],
};

#[cfg(test)]
mod tests {}
