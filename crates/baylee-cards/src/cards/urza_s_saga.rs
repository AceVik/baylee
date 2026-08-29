//! Urza's Saga — (no cost) — Enchantment Land — Urza's Saga
//! Oracle: (As this Saga enters and after your draw step, add a lore counter. Sacrifice after III.)
//! Oracle: I — This Saga gains "{T}: Add {C}."
//! Oracle: II — This Saga gains "{2}, {T}: Create a 0/0 colorless Construct artifact creature token with 'This token gets +1/+1 for each artifact you control.'"
//! Oracle: III — Search your library for an artifact card with mana cost {0} or {1}, put it onto the battlefield, then shuffle.
//! Set: MH2 #259 — Modern Horizons 2 | Scryfall ID: c1e0f201-42cb-46a1-901a-65bb4fc18f6c | Oracle ID: 4c6a0c30-b547-4eff-8ff4-0ca25803c076
// IMPLEMENTED — full saga: chapter I grants {T}: Add {C} (the baseline
// mana ability covers it), chapter II grants the Construct ability
// (GrantActivated + per-artifact P/T token), chapter III tutors a 0- or
// 1-cost artifact to the battlefield.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, ActivationZone, CardDef, CommanderRule, Cost, Coverage, Effect,
    FaceDef, Filter, KeywordSet, Layer, Modifier, PartnerKind, SearchDest, TokenDef,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature, enchantment};
use baylee_core::ids::CardIndex;
use baylee_core::mana::{ManaColor, ManaCost};
use baylee_core::types::{SupertypeSet, TypeSet};

static CONSTRUCT: TokenDef = TokenDef {
    name: "Construct",
    colors: ColorSet::EMPTY,
    types: TypeSet::CREATURE.union(TypeSet::ARTIFACT),
    supertypes: SupertypeSet::EMPTY,
    subtypes: &[creature::CONSTRUCT],
    power: Some(0),
    toughness: Some(0),
    keywords: KeywordSet::EMPTY,
};
static ARTIFACT_F: Filter = Filter::HasType(TypeSet::ARTIFACT);
static ARTIFACT_CMC1: Filter =
    Filter::And(&[Filter::HasType(TypeSet::ARTIFACT), Filter::CmcAtMost(1)]);
static CHAPTER_I_FX: &[Effect] = &[Effect::AddMana {
    color: ManaColor::Colorless,
    amount: 1,
}];

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(179),
    oracle_id: "4c6a0c30-b547-4eff-8ff4-0ca25803c076",
    scryfall_id: "c1e0f201-42cb-46a1-901a-65bb4fc18f6c",
    faces: &[FaceDef {
        name: "Urza's Saga",
        mana_cost: ManaCost::ZERO,
        types: TypeSet::LAND.union(TypeSet::ENCHANTMENT),
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[enchantment::SAGA],
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
    }],
    color_identity: ColorSet::EMPTY,
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[
        // Chapter I's granted "{T}: Add {C}" is this baseline mana
        // ability — it covers the same text (CR 714.3a grants it
        // permanently, so the approximation is exact from chapter I on).
        AbilityDef::Activated {
            cost: Cost::TAP,
            effects: &[Effect::AddMana {
                color: ManaColor::Colorless,
                amount: 1,
            }],
            target: None,
            timing: ActivationTiming::InstantSpeed,
            mana_ability: true,
            zone: ActivationZone::Battlefield,
        },
        AbilityDef::SagaChapter {
            chapter: 1,
            effects: &[Effect::CreateContinuousEffect {
                layer: Layer::Ability,
                filter: &Filter::This,
                modifier: Modifier::GrantActivated {
                    cost: Cost::TAP,
                    effects: CHAPTER_I_FX,
                    mana_ability: true,
                },
                duration: baylee_cards_dsl::Duration::WhileSourceOnBattlefield,
            }],
            target: None,
        },
        AbilityDef::SagaChapter {
            chapter: 2,
            effects: &[Effect::CreateContinuousEffect {
                layer: Layer::Ability,
                filter: &Filter::This,
                modifier: Modifier::GrantActivated {
                    cost: Cost {
                        mana: baylee_core::mana!("{2}"),
                        parts: &[baylee_cards_dsl::CostPart::TapSelf],
                    },
                    effects: &[Effect::CreateTokenPtPerCount {
                        token: &CONSTRUCT,
                        filter: &ARTIFACT_F,
                        p: 1,
                        t: 1,
                    }],
                    mana_ability: false,
                },
                duration: baylee_cards_dsl::Duration::WhileSourceOnBattlefield,
            }],
            target: None,
        },
        AbilityDef::SagaChapter {
            chapter: 3,
            effects: &[Effect::SearchLibrary {
                filter: &ARTIFACT_CMC1,
                dest: SearchDest::Battlefield,
                tapped: false,
                shuffle: true,
                optional: false,
            }],
            target: None,
        },
    ],
};

#[cfg(test)]
mod tests {}
