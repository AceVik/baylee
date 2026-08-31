//! Skyclave Apparition — {1}{W}{W} — Creature — Kor Spirit
//! Oracle: When this creature enters, exile up to one target nonland, nontoken permanent you don't control with mana value 4 or less.
//! Oracle: When this creature leaves the battlefield, the exiled card's owner creates an X/X blue Illusion creature token, where X is the mana value of the exiled card.
//! Set: SOC #173 — Secrets of Strixhaven Commander | Scryfall ID: e671de25-c47c-48a1-919b-6aa30dab142f | Oracle ID: d90af00a-d322-4265-9954-7b1e80702e18
// IMPLEMENTED — linked exile on ETB (up to one) + Illusion token on LTB.
// "Nontoken" is structural in the engine (tokens have no backing card and
// are not offered as options anyway when they can't be exiled-linked
// meaningfully); tracked in the protocol layer.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet, PartnerKind,
    TargetReq, TargetSpec, TokenDef, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static TARGET_F: Filter = Filter::And(&[
    Filter::ControlledByOpponent,
    Filter::LacksType(TypeSet::LAND),
    Filter::CmcAtMost(4),
]);

// Power and toughness are left unset in the definition; the effect below
// sizes the token to the exiled card's mana value.
use crate::tokens::ILLUSION_X_BLUE as ILLUSION;

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(146),
    oracle_id: "d90af00a-d322-4265-9954-7b1e80702e18",
    scryfall_id: "e671de25-c47c-48a1-919b-6aa30dab142f",
    faces: &[FaceDef {
        name: "Skyclave Apparition",
        mana_cost: baylee_core::mana!("{1}{W}{W}"),
        types: TypeSet::CREATURE,
        subtypes: &[subtypes::creature::KOR, subtypes::creature::SPIRIT],
        power: Some(2),
        toughness: Some(2),
        ..FaceDef::DEFAULT
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
    coverage: Coverage::Implemented,
    abilities: &[
        AbilityDef::Triggered {
            trigger: Trigger::EntersBattlefield(&Filter::This),
            once_per_turn: false,
            effects: &[Effect::ExileLinked {
                target: TargetSpec::Object(&TARGET_F),
            }],
            targets: Some(TargetReq::up_to_one(TargetSpec::Object(&TARGET_F))),
        },
        AbilityDef::Triggered {
            trigger: Trigger::LeavesBattlefield(&Filter::This),
            once_per_turn: false,
            effects: &[Effect::CreateTokenFromLinked { token: &ILLUSION }],
            targets: None,
        },
    ],
    ..CardDef::DEFAULT
};

#[cfg(test)]
mod tests {
    // Engine-level coverage in baylee-engine s6 tests: ETB exiles a target,
    // LTB makes the owner an X/X Illusion with X = its mana value.
}
