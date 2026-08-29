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

static ANY_CREATURE: Filter = Filter::HasType(TypeSet::CREATURE);

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
        power: None,
        toughness: None,
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
        abilities: &[],
        castable_from_hand: true,
    }],
    color_identity: ColorSet::EMPTY,
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::Legendary,
    partner: PartnerKind::None,
    coverage: Coverage::Partial(
        "token isn't legendary + gains haste (token modification, M2.S7c+)",
    ),
    abilities: &[
        AbilityDef::Triggered {
            trigger: Trigger::StepBegin {
                step: StepKind::CombatBegin,
                whose: baylee_cards_dsl::PlayerRel::You,
            },
            once_per_turn: false,
            effects: &[Effect::CreateTokenCopyOfEquipped { kicked_bonus: 0 }],
            targets: None,
        },
        AbilityDef::Activated {
            cost: Cost {
                mana: baylee_core::mana!("{5}"),
                parts: &[],
            },
            effects: &[Effect::AttachSelf {
                target: TargetSpec::Object(&ANY_CREATURE),
            }],
            target: Some(TargetSpec::Object(&ANY_CREATURE)),
            timing: ActivationTiming::SorcerySpeed,
            mana_ability: false,
            zone: ActivationZone::Battlefield,
        },
    ],
};

#[cfg(test)]
mod tests {}
