//! Luminarch Ascension — {1}{W} — Enchantment
//! Oracle: At the beginning of each opponent's end step, if you didn't lose life this turn, you may put a quest counter on this enchantment. (Damage causes loss of life.)
//! Oracle: {1}{W}: Create a 4/4 white Angel creature token with flying. Activate only if this enchantment has four or more quest counters on it.
//! Set: ZEN #25 — Zendikar | Scryfall ID: b3770d86-4496-4c06-aab1-2917cfec100e | Oracle ID: 90076bf5-aa9a-4a6e-9035-9aa97fd5561e
// PARTIAL — quest counters accumulate via the end-step trigger (life-loss
// condition evaluated from the journal). The angel-token ability needs
// activation gating by counters (M2+); currently always activatable.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, ActivationZone, Amount, CardDef, CommanderRule, Cost,
    CounterKind, Coverage, Effect, FaceDef, KeywordSet, PartnerKind, StepKind, TokenDef, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static ANGEL: TokenDef = TokenDef {
    name: "Angel",
    colors: ColorSet::from_slice(&[Color::White]),
    types: TypeSet::CREATURE,
    supertypes: SupertypeSet::EMPTY,
    subtypes: &[creature::ANGEL],
    power: Some(4),
    toughness: Some(4),
    keywords: KeywordSet::FLYING,
};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(88),
    oracle_id: "90076bf5-aa9a-4a6e-9035-9aa97fd5561e",
    scryfall_id: "b3770d86-4496-4c06-aab1-2917cfec100e",
    faces: &[FaceDef {
        name: "Luminarch Ascension",
        mana_cost: baylee_core::mana!("{1}{W}"),
        types: TypeSet::ENCHANTMENT,
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
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Partial(
        "angel ability needs counter-gated activation (M2+); quest counters work",
    ),
    abilities: &[
        AbilityDef::Triggered {
            trigger: Trigger::StepBegin {
                step: StepKind::End,
                whose: baylee_cards_dsl::PlayerRel::Opponent,
            },
            once_per_turn: false,
            effects: &[Effect::AddCounter {
                kind: CounterKind::Custom(1),
                amount: Amount::Fixed(1),
            }],
            targets: None,
        },
        AbilityDef::Activated {
            cost: Cost {
                mana: baylee_core::mana!("{1}{W}"),
                parts: &[],
            },
            effects: &[Effect::CreateToken { token: &ANGEL }],
            target: None,
            timing: ActivationTiming::InstantSpeed,
            mana_ability: false,
            zone: ActivationZone::Battlefield,
        },
    ],
};

#[cfg(test)]
mod tests {}
