//! Sword of Hearth and Home — {3} — Artifact — Equipment
//! Oracle: Equipped creature gets +2/+2 and has protection from green and from white.
//! Oracle: Whenever equipped creature deals combat damage to a player, exile up to one target creature you own, then search your library for a basic land card. Put both cards onto the battlefield under your control, then shuffle.
//! Oracle: Equip {2}
//! Set: MH2 #238 — Modern Horizons 2 | Scryfall ID: a16fabbe-4557-4067-b882-f2e5dbd8b458 | Oracle ID: 913e6182-706a-4872-8c8a-e146b0ae0738
// IMPLEMENTED — +2/+2, protection from green/white, the blink+ramp
// damage trigger, and equip.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, ActivationZone, CardDef, CommanderRule, Cost, Coverage, Effect,
    FaceDef, Filter, KeywordSet, Layer, Modifier, PartnerKind, SearchDest, StaticAbility,
    TargetReq, TargetSpec, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, artifact};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static EQUIPPED: Filter = Filter::AttachedToBySource;
static GREEN_F: Filter = Filter::HasColor(ColorSet::from_slice(&[Color::Green]));
static WHITE_F: Filter = Filter::HasColor(ColorSet::from_slice(&[Color::White]));
static CREATURE_YOU_OWN: Filter =
    Filter::And(&[Filter::HasType(TypeSet::CREATURE), Filter::OwnedByYou]);
static BASIC_LAND: Filter = Filter::And(&[
    Filter::HasSupertype(SupertypeSet::BASIC),
    Filter::HasType(TypeSet::LAND),
]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(163),
    oracle_id: "913e6182-706a-4872-8c8a-e146b0ae0738",
    scryfall_id: "a16fabbe-4557-4067-b882-f2e5dbd8b458",
    faces: &[FaceDef {
        name: "Sword of Hearth and Home",
        mana_cost: baylee_core::mana!("{3}"),
        types: TypeSet::ARTIFACT,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[artifact::EQUIPMENT],
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
    }],
    color_identity: ColorSet::EMPTY,
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[
        AbilityDef::Static(StaticAbility {
            layer: Layer::PtModify,
            filter: Filter::AttachedToBySource,
            modifier: Modifier::ModifyPT(2, 2),
            cross_zone: false,
        }),
        AbilityDef::Static(StaticAbility {
            layer: Layer::Text,
            filter: Filter::AttachedToBySource,
            modifier: Modifier::ProtectionFrom(&GREEN_F),
            cross_zone: false,
        }),
        AbilityDef::Static(StaticAbility {
            layer: Layer::Text,
            filter: Filter::AttachedToBySource,
            modifier: Modifier::ProtectionFrom(&WHITE_F),
            cross_zone: false,
        }),
        AbilityDef::Triggered {
            trigger: Trigger::DealsCombatDamageToPlayer(&EQUIPPED),
            once_per_turn: false,
            effects: &[
                Effect::Blink {
                    target: TargetSpec::Object(&CREATURE_YOU_OWN),
                },
                Effect::SearchLibrary {
                    filter: &BASIC_LAND,
                    dest: SearchDest::Battlefield,
                    tapped: false,
                    shuffle: true,
                    optional: true,
                },
            ],
            targets: Some(TargetReq {
                spec: TargetSpec::Object(&CREATURE_YOU_OWN),
                min: 0,
                max: 1,
                count_is_x: false,
            }),
        },
        AbilityDef::Activated {
            cost: baylee_cards_dsl::Cost {
                mana: baylee_core::mana!("{2}"),
                parts: &[],
            },
            effects: &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::HasType(TypeSet::CREATURE)),
            }],
            target: Some(TargetSpec::Object(&Filter::HasType(TypeSet::CREATURE))),
            timing: ActivationTiming::SorcerySpeed,
            mana_ability: false,
            zone: ActivationZone::Battlefield,
        },
    ],
};

#[cfg(test)]
mod tests {}
