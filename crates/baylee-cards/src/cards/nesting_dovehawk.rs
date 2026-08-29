//! Nesting Dovehawk — {2}{W} — Creature — Bird
//! Oracle: Flying
//! Oracle: At the beginning of combat on your turn, populate. (Create a token that's a copy of a creature token you control.)
//! Oracle: Whenever a creature token you control enters, put a +1/+1 counter on this creature.
//! Set: EOC #25 — Edge of Eternities Commander | Scryfall ID: c58ff93f-7135-40af-92ce-358da48694dc | Oracle ID: fe8fc442-ed17-40b2-8624-69f2eed3f9be
// PARTIAL — flying + counter on creature-token ETBs implemented. Populate
// approximated: creates a token copy of one of your creature tokens when
// one exists (populate's choose-a-token and token-only restriction are
// noted; currently no-op without a creature token).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, Amount, CardDef, CommanderRule, CounterKind, Coverage, Effect, FaceDef, Filter,
    KeywordSet, PartnerKind, StepKind, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static YOUR_CREATURE_ETB: Filter = Filter::And(&[
    Filter::ControlledByYou,
    Filter::HasType(TypeSet::CREATURE),
    Filter::Another,
]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(103),
    oracle_id: "fe8fc442-ed17-40b2-8624-69f2eed3f9be",
    scryfall_id: "c58ff93f-7135-40af-92ce-358da48694dc",
    faces: &[FaceDef {
        name: "Nesting Dovehawk",
        mana_cost: baylee_core::mana!("{2}{W}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[creature::BIRD],
        power: Some(2),
        toughness: Some(2),
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
    keywords: KeywordSet::FLYING,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Partial(
        "populate chooses a creature token (token-only target, M2+); approximated as copy-of-first-creature-token",
    ),
    abilities: &[
        AbilityDef::Triggered {
            trigger: Trigger::StepBegin {
                step: StepKind::CombatBegin,
                whose: baylee_cards_dsl::PlayerRel::You,
            },
            once_per_turn: false,
            effects: &[Effect::CreateTokenCopyOfFirstToken],
            targets: None,
        },
        AbilityDef::Triggered {
            trigger: Trigger::EntersBattlefield(&YOUR_CREATURE_ETB),
            once_per_turn: false,
            effects: &[Effect::AddCounter {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(1),
            }],
            targets: None,
        },
    ],
};

#[cfg(test)]
mod tests {}
