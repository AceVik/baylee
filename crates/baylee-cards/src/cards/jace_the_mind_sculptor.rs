//! Jace, the Mind Sculptor — {2}{U}{U} — Legendary Planeswalker — Jace
//! Oracle: +2: Look at the top card of target player's library. You may put that card on the bottom of that player's library.
//! Oracle: 0: Draw three cards, then put two cards from your hand on top of your library in any order.
//! Oracle: −1: Return target creature to its owner's hand.
//! Oracle: −12: Exile all cards from target player's library, then that player shuffles their hand into their library.
//! Set: 2XM #56 — Double Masters | Scryfall ID: c8817585-0d32-4d56-9142-0d29512e86a9 | Oracle ID: 7f77a84e-5a4b-4834-aefa-3cecc175ae8e
// IMPLEMENTED — all four loyalty abilities.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, Amount, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet,
    PartnerKind, PlayerRel, TargetReq, TargetSpec,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, planeswalker};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static CREATURE_F: Filter = Filter::HasType(TypeSet::CREATURE);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(76),
    oracle_id: "7f77a84e-5a4b-4834-aefa-3cecc175ae8e",
    scryfall_id: "c8817585-0d32-4d56-9142-0d29512e86a9",
    faces: &[FaceDef {
        name: "Jace, the Mind Sculptor",
        mana_cost: baylee_core::mana!("{2}{U}{U}"),
        types: TypeSet::PLANESWALKER,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[planeswalker::JACE],
        loyalty: Some(3),
        ..FaceDef::DEFAULT
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    commander: CommanderRule::Legendary,
    coverage: Coverage::Implemented,
    abilities: &[
        AbilityDef::Loyalty {
            cost: 2,
            effects: &[Effect::ScryFor {
                player: PlayerRel::Chosen,
                amount: Amount::Fixed(1),
            }],
            target: Some(TargetSpec::AnyPlayer),
        },
        AbilityDef::Loyalty {
            cost: 0,
            effects: &[
                Effect::DrawCards {
                    amount: Amount::Fixed(3),
                },
                Effect::PutFromHandOnTop { count: 2 },
            ],
            target: None,
        },
        AbilityDef::Loyalty {
            cost: -1,
            effects: &[Effect::ReturnToHand {
                target: TargetSpec::Object(&CREATURE_F),
            }],
            target: Some(TargetSpec::Object(&CREATURE_F)),
        },
        AbilityDef::Loyalty {
            cost: -12,
            effects: &[Effect::ExileLibraryAndShuffleHand {
                player: PlayerRel::Chosen,
            }],
            target: Some(TargetSpec::AnyPlayer),
        },
    ],
    ..CardDef::DEFAULT
};

#[cfg(test)]
mod tests {}
