//! Emeritus of Woe — {3}{B} — Creature — Vampire Warlock
//! Oracle: This creature enters prepared. (While it's prepared, you may cast a copy of its spell. Doing so unprepares it.)
//! Oracle: At the beginning of your end step, if two or more creatures died this turn, this creature becomes prepared.
//! (Its spell: Demonic Tutor — {1}{B} — Sorcery: Search your library for a card, put that card into your hand, then shuffle.)
//! Set: MH2 #92 — Modern Horizons 2 | Scryfall ID: 7eb9e83d-515d-4911-a06b-9982200277b2 | Oracle ID: 93056597-b964-421f-be2f-e92abef1c2a4
// IMPLEMENTED — the real prepared mechanic: enters prepared (cast a copy
// of Demonic Tutor from the registry while prepared, unpreparing it),
// and re-prepares at your end step when 2+ creatures died this turn.
// NOTE: an earlier version of this file invented an MDFC back face —
// that was wrong data; the card is the prepared Vampire Warlock above.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, Effect, EnterModifier, FaceDef, Filter,
    KeywordSet, PartnerKind, StepKind, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

/// The linked spell: Demonic Tutor (registry card).
static DEMONIC_TUTOR: CardIndex = CardIndex::new(32);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(41),
    oracle_id: "93056597-b964-421f-be2f-e92abef1c2a4",
    scryfall_id: "7eb9e83d-515d-4911-a06b-9982200277b2",
    faces: &[FaceDef {
        name: "Emeritus of Woe",
        mana_cost: baylee_core::mana!("{3}{B}"),
        types: TypeSet::CREATURE,
        subtypes: &[creature::VAMPIRE, creature::WARLOCK],
        power: Some(5),
        toughness: Some(4),
        enter_modifiers: &[EnterModifier::Prepared],
        ..FaceDef::DEFAULT
    }],
    color_identity: ColorSet::from_slice(&[Color::Black]),
    keywords: KeywordSet::FLYING,
    coverage: Coverage::Implemented,
    abilities: &[
        AbilityDef::Prepared {
            card: DEMONIC_TUTOR,
        },
        AbilityDef::Triggered {
            trigger: Trigger::StepBegin {
                step: StepKind::End,
                whose: baylee_cards_dsl::PlayerRel::You,
            },
            once_per_turn: false,
            effects: &[Effect::IfCreaturesDiedAtLeast {
                n: 2,
                then: &[Effect::BecomePrepared],
            }],
            targets: None,
        },
    ],
    ..CardDef::DEFAULT
};

#[cfg(test)]
mod tests {}
