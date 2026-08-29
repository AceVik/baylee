//! Loran of the Third Path — {2}{W} — Legendary Creature — Human Artificer
//! Oracle: Vigilance
//! Oracle: When Loran enters, destroy up to one target artifact or enchantment.
//! Oracle: {T}: You and target opponent each draw a card.
//! Set: MKC #71 — Murders at Karlov Manor Commander | Scryfall ID: 9e83a0ef-4fea-45ba-86c0-130d6687f7fe | Oracle ID: b3d81980-76f2-44e2-b1c9-01e30c726312
// IMPLEMENTED — vigilance, ETB destroy, tap-draw for you and an opponent.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, ActivationZone, Amount, CardDef, CommanderRule, Cost, Coverage,
    Effect, FaceDef, Filter, KeywordSet, PartnerKind, PlayerRel, TargetReq, TargetSpec, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static ARTIFACT_OR_ENCHANTMENT: Filter = Filter::Or(&[
    Filter::HasType(TypeSet::ARTIFACT),
    Filter::HasType(TypeSet::ENCHANTMENT),
]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(87),
    oracle_id: "b3d81980-76f2-44e2-b1c9-01e30c726312",
    scryfall_id: "9e83a0ef-4fea-45ba-86c0-130d6687f7fe",
    faces: &[FaceDef {
        name: "Loran of the Third Path",
        mana_cost: baylee_core::mana!("{2}{W}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[creature::HUMAN, creature::ARTIFICER],
        power: Some(2),
        toughness: Some(1),
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
    keywords: KeywordSet::VIGILANCE,
    commander: CommanderRule::Legendary,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[
        AbilityDef::Triggered {
            trigger: Trigger::EntersBattlefield(&Filter::This),
            once_per_turn: false,
            effects: &[Effect::Destroy {
                target: TargetSpec::Object(&ARTIFACT_OR_ENCHANTMENT),
            }],
            targets: Some(TargetReq::up_to_one(TargetSpec::Object(
                &ARTIFACT_OR_ENCHANTMENT,
            ))),
        },
        AbilityDef::Activated {
            cost: Cost::TAP,
            effects: &[
                Effect::DrawCards {
                    amount: Amount::Fixed(1),
                },
                Effect::DrawCardsFor {
                    amount: Amount::Fixed(1),
                    who: PlayerRel::Opponent,
                },
            ],
            target: None,
            timing: ActivationTiming::InstantSpeed,
            mana_ability: false,
            zone: ActivationZone::Battlefield,
        },
    ],
};

#[cfg(test)]
mod tests {}
