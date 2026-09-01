//! Helm of the Host — {4} — Legendary Artifact — Equipment
//! Oracle: At the beginning of combat on your turn, create a token that's a copy of equipped creature, except the token isn't legendary. That token gains haste.
//! Oracle: Equip {5}
//! Set: MSC #200 — Marvel Super Heroes Commander | Scryfall ID: 70ffc71f-328d-421d-926b-6f2e45ffb812 | Oracle ID: 83b43aba-bf9c-4da2-967d-9daa632e97d2
// IMPLEMENTED — equipment + combat-begin token copy of the equipped creature.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, ActivationZone, CardDef, CommanderRule, Cost, Coverage, Effect,
    FaceDef, Filter, KeywordSet, PartnerKind, StepKind, TargetSpec, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, artifact};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

/// Equip targets "target creature you control" (CR 702.6a).
static CREATURE_YOU_CONTROL: Filter =
    Filter::And(&[Filter::HasType(TypeSet::CREATURE), Filter::ControlledByYou]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(68),
    oracle_id: "83b43aba-bf9c-4da2-967d-9daa632e97d2",
    scryfall_id: "70ffc71f-328d-421d-926b-6f2e45ffb812",
    faces: &[FaceDef {
        name: "Helm of the Host",
        mana_cost: baylee_core::mana!("{4}"),
        types: TypeSet::ARTIFACT,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[artifact::EQUIPMENT],
        ..FaceDef::DEFAULT
    }],
    commander: CommanderRule::Legendary,
    coverage: Coverage::Implemented,
    abilities: &[
        AbilityDef::Triggered {
            trigger: Trigger::StepBegin {
                step: StepKind::CombatBegin,
                whose: baylee_cards_dsl::PlayerRel::You,
            },
            once_per_turn: false,
            effects: &[Effect::CreateTokenCopyOfEquipped {
                kicked_bonus: 0,
                mods: &[
                    baylee_cards_dsl::CopyMod::RemoveSupertype(SupertypeSet::LEGENDARY),
                    baylee_cards_dsl::CopyMod::AddKeyword(KeywordSet::HASTE),
                ],
            }],
            targets: None,
        },
        AbilityDef::Activated {
            cost: Cost {
                mana: baylee_core::mana!("{5}"),
                parts: &[],
            },
            effects: &[Effect::AttachSelf {
                target: TargetSpec::Object(&CREATURE_YOU_CONTROL),
            }],
            target: Some(TargetSpec::Object(&CREATURE_YOU_CONTROL)),
            timing: ActivationTiming::SorcerySpeed,
            mana_ability: false,
            zone: ActivationZone::Battlefield,
        },
    ],
    ..CardDef::DEFAULT
};
