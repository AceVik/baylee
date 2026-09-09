//! Sword of Hearth and Home — {3} — Artifact — Equipment
//! Oracle: Equipped creature gets +2/+2 and has protection from green and from white.
//! Oracle: Whenever equipped creature deals combat damage to a player, exile up to one target creature you own, then search your library for a basic land card. Put both cards onto the battlefield under your control, then shuffle.
//! Oracle: Equip {2}
//! Set: MH2 #238 — Modern Horizons 2 | Scryfall ID: a16fabbe-4557-4067-b882-f2e5dbd8b458 | Oracle ID: 913e6182-706a-4872-8c8a-e146b0ae0738
// IMPLEMENTED — +2/+2, protection from green/white, the blink+ramp
// damage trigger, and equip.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::artifact;

/// Equip targets "target creature you control" (CR 702.6a).
static CREATURE_YOU_CONTROL: Filter = Filter::And(&[Filter::CREATURE, Filter::ControlledByYou]);
static GREEN_F: Filter = Filter::HasColor(ColorSet::from_slice(&[Color::Green]));
static WHITE_F: Filter = Filter::HasColor(ColorSet::from_slice(&[Color::White]));
static CREATURE_YOU_OWN: Filter = Filter::And(&[Filter::CREATURE, Filter::OwnedByYou]);
static BASIC_LAND: Filter = Filter::And(&[Filter::HasSupertype(SupertypeSet::BASIC), Filter::LAND]);

card! {
    index: 163,
    oracle_id: "913e6182-706a-4872-8c8a-e146b0ae0738",
    scryfall_id: "a16fabbe-4557-4067-b882-f2e5dbd8b458",
    faces: &[face! {
        name: "Sword of Hearth and Home",
        mana_cost: baylee_core::mana!("{3}"),
        types: TypeSet::ARTIFACT,
        subtypes: &[artifact::EQUIPMENT],
    }],
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
        triggered!(Trigger::DealsCombatDamageToPlayer(&Filter::AttachedToBySource), &[
                Effect::Blink {
                    target: TargetSpec::Object(&CREATURE_YOU_OWN),
                },
                Effect::SearchLibrary {
                    filter: &BASIC_LAND,
                    finds: &[Find::BATTLEFIELD],
                    optional: true,
                },
            ], targets: Some(TargetReq {
                spec: TargetSpec::Object(&CREATURE_YOU_OWN),
                min: 0,
                max: 1,
                count_is_x: false,
            })),
        activated!(Cost {
                mana: baylee_core::mana!("{2}"),
                parts: &[],
            }, &[Effect::AttachSelf {
                target: TargetSpec::Object(&CREATURE_YOU_CONTROL),
            }], target: Some(TargetSpec::Object(&CREATURE_YOU_CONTROL)), timing: ActivationTiming::SorcerySpeed),
    ],
}
