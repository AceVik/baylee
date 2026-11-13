//! Total War — {3}{R} — Enchantment
//! Oracle: Whenever a player attacks with one or more creatures, destroy all untapped non-Wall creatures that player controls that didn't attack, except for creatures the player hasn't controlled continuously since the beginning of the turn.
//! Set: ICE #221 — Ice Age | Scryfall ID: 6107388b-ec1e-401e-a407-a821c908ed8d | Oracle ID: 8266dd1c-e262-4467-a893-6b3b5eedb427

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static IDLE_NON_WALL: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Untapped,
    Filter::Not(&Filter::HasSubtype(subtypes::creature::WALL)),
    Filter::ControlledByActivePlayer,
    Filter::ControlledSinceTurnBegan,
    Filter::Not(&Filter::AttackedThisTurn),
]);

card!(
    index = index::TOTAL_WAR,
    oracle_id = "8266dd1c-e262-4467-a893-6b3b5eedb427",
    scryfall_id = "6107388b-ec1e-401e-a407-a821c908ed8d",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Total War",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::OneOrMoreAttack(&Filter::CREATURE),
        &[Effect::destroy_all(&IDLE_NON_WALL)]
    )],
);
