//! Aminatou, the Fateshifter — {W}{U}{B} — Legendary Planeswalker — Aminatou
//! Oracle: +1: Draw a card, then put a card from your hand on top of your library.
//! Oracle: −1: Exile another target permanent you own, then return it to the battlefield under your control.
//! Oracle: −6: Choose left or right. Each player gains control of all nonland permanents other than Aminatou controlled by the next player in the chosen direction.
//! Oracle: Aminatou, the Fateshifter can be your commander.
//! Set: 2X2 #169 — Double Masters 2022 | Scryfall ID: bc010302-e715-4946-89eb-a214e0b836ba | Oracle ID: 3a30089d-cd2d-49be-9b06-7a2454117692
// PARTIAL — +1 and −1 implemented; −6 needs directional multiplayer control
// rotation (M2+; heads-up it is a straight swap, still unimplemented).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, Amount, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet,
    PartnerKind, TargetReq, TargetSpec,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, planeswalker};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static OWNED_PERMANENT: Filter = Filter::And(&[Filter::OwnedByYou, Filter::Another]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(3),
    oracle_id: "3a30089d-cd2d-49be-9b06-7a2454117692",
    scryfall_id: "bc010302-e715-4946-89eb-a214e0b836ba",
    faces: &[FaceDef {
        name: "Aminatou, the Fateshifter",
        mana_cost: baylee_core::mana!("{W}{U}{B}"),
        types: TypeSet::PLANESWALKER,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[planeswalker::AMINATOU],
        loyalty: Some(3),
        ..FaceDef::DEFAULT
    }],
    color_identity: ColorSet::from_slice(&[Color::White, Color::Blue, Color::Black]),
    commander: CommanderRule::ExplicitlyAllowed,
    coverage: Coverage::Implemented,
    abilities: &[
        AbilityDef::Loyalty {
            cost: 1,
            effects: &[
                Effect::DrawCards {
                    amount: Amount::Fixed(1),
                },
                Effect::PutFromHandOnTop { count: 1 },
            ],
            target: None,
        },
        AbilityDef::Loyalty {
            cost: -1,
            effects: &[Effect::Blink {
                target: TargetSpec::Object(&OWNED_PERMANENT),
            }],
            target: Some(TargetSpec::Object(&OWNED_PERMANENT)),
        },
        AbilityDef::Loyalty {
            cost: -6,
            effects: &[Effect::ControlRotation],
            target: None,
        },
    ],
    ..CardDef::DEFAULT
};

#[cfg(test)]
mod tests {}
