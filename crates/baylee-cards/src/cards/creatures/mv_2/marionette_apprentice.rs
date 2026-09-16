//! Marionette Apprentice — {1}{B} — Creature — Human Artificer
//! Oracle: Fabricate 1 (When this creature enters, put a +1/+1 counter on it or create a 1/1 colorless Servo artifact creature token.)
//! Oracle: Whenever another creature or artifact you control is put into a graveyard from the battlefield, each opponent loses 1 life.
//! Set: MH3 #100 — Modern Horizons 3 | Scryfall ID: d16f8670-f038-400a-83e7-a53a7f8c47ac | Oracle ID: 726d9d2c-736a-4852-9938-a0f50d8fd89f
// PARTIAL — a drain engine: every other creature or artifact you control that
// goes from the battlefield to a graveyard takes 1 life from each opponent.
//
// NOT SUPPORTED: `Fabricate 1` — "put a +1/+1 counter on it or create a 1/1
// colorless Servo artifact creature token". The trigger and the choice are
// both sayable — `modal_triggered!(Trigger::ETB, …)` with two `mode!`s — and
// the counter half is `Effect::AddCounter`. The token half is not: there is
// no Servo in `tokens::ALL` for `Effect::CreateToken` to name, and a
// `TokenDef` written in a card file instead reaches the table with no art key
// at all, which `tokens`' `no_card_file_defines_its_own_token` is a build
// failure about. Half a "choose one" would be the worse card — a mandatory
// counter the player never agreed to — so the whole clause is dropped and the
// Apprentice plays exactly as though the line were not printed: a 1/2 for
// {1}{B} that drains.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// "another creature or artifact you control" — whose trip to a graveyard the
/// drain reads. Noun first, then `your`, then `another`, which is the clause
/// order `Filter::ANOTHER_CREATURE_YOU_CONTROL` fixes for this shape.
static ANOTHER_ARTIFACT_OR_CREATURE_YOU_CONTROL: Filter = Filter::And(&[
    Filter::ARTIFACT_OR_CREATURE,
    Filter::ControlledByYou,
    Filter::Another,
]);

card!(
    index = index::MARIONETTE_APPRENTICE,
    oracle_id = "726d9d2c-736a-4852-9938-a0f50d8fd89f",
    scryfall_id = "d16f8670-f038-400a-83e7-a53a7f8c47ac",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial("fabricate 1 is not written: no Servo token is created"),
    faces = &[face!(
        name = "Marionette Apprentice",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::ARTIFICER],
        power = Some(1),
        toughness = Some(2),
    ),],
    // "is put into a graveyard from the battlefield" is what `Trigger::Dies`
    // matches — a battlefield → graveyard zone change for *any* object, not
    // only a creature, which is what lets one trigger read both halves of
    // "creature or artifact". `EachOpponent` and not `Opponent`: the card
    // takes the life from every one of them at a table of four.
    abilities = &[triggered!(
        Trigger::Dies(&ANOTHER_ARTIFACT_OR_CREATURE_YOU_CONTROL),
        &[Effect::LoseLife {
            amount: Amount::Fixed(1),
            target: PlayerRel::EachOpponent,
        }]
    )],
);

// Behaviour belongs in an engine test (crates/baylee-engine/src/engine/card_tests.rs):
// one of your artifacts dying drains as readily as one of your creatures, an
// opponent's dying permanent does not, and the Apprentice's own death does
// not — `Another` is what says so.
