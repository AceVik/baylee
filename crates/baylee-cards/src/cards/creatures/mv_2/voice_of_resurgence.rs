//! Voice of Resurgence — {G}{W} — Creature — Elemental
//! Oracle: Whenever an opponent casts a spell during your turn and when this creature dies, create a green and white Elemental creature token with "This token's power and toughness are each equal to the number of creatures you control."
//! Set: 2XM #227 — Double Masters | Scryfall ID: 99d1e843-71c9-4a65-bc36-d23858ef5ead | Oracle ID: 5cbbb3f3-63a4-4983-81ea-8c405b10e63f
// IMPLEMENTED — one sentence, two trigger events: an opponent's spell cast
// during your turn, and this creature dying. Each makes the */* Elemental
// whose characteristic-defining ability counts the creatures you control.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

use crate::tokens;

const MAKE_ELEMENTAL: &[Effect] = &[Effect::CreateToken {
    token: &tokens::ELEMENTAL_X_X_GREEN_WHITE,
}];

card!(
    index = index::VOICE_OF_RESURGENCE,
    oracle_id = "5cbbb3f3-63a4-4983-81ea-8c405b10e63f",
    scryfall_id = "99d1e843-71c9-4a65-bc36-d23858ef5ead",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    faces = &[face!(
        name = "Voice of Resurgence",
        mana_cost = mana!("{G}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELEMENTAL],
        power = Some(2),
        toughness = Some(2),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        // "Whenever an opponent casts a spell during your turn": the turn is
        // read as the trigger fires and again as it resolves, and nothing
        // can end a turn between the two.
        triggered!(
            Trigger::SpellCast(&Filter::ControlledByOpponent),
            MAKE_ELEMENTAL,
            condition = Some(Condition::YourTurn)
        ),
        // "and when this creature dies" (CR 700.4).
        triggered!(Trigger::Dies(&Filter::This), MAKE_ELEMENTAL),
    ],
);
