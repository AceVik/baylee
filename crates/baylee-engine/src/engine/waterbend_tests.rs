//! Waterbend, and the table that stopped answering.
//!
//! Reported from a live game: "Die eine Karte mit Waterbend 6 - das
//! Waterband war komplett kaputt. Sie wollte von mir 99 Targets, hat 5 Mana
//! gekostet und als ich keine Targets bestimmen konnte, da kam nur die
//! Meldung für invalid targets oder costs und ab hier war das Spiel gar
//! nicht mehr spielbar."
//!
//! Waterbend is an *optional* additional cost paid convoke-style, so the
//! cast asks two questions: "waterbend {6}?", then "which permanents do you
//! tap to help". Saying yes to the first and nothing to the second leaves
//! `{10}{U}` against five Islands, which is unpayable — a legal line to walk
//! into, and one the engine has to refuse.
//!
//! The last clause of the report is what the refusal did wrong. Answering it
//! took measuring: `Engine::apply` propagates `apply_inner`'s error before
//! `run_until_choice`, which looks like the cause and is not — the wizard's
//! own error branch already resumed the game. What it resumed *into* was the
//! next seat's priority.

use super::testkit::{Duel, RegistryLookup, card_index, keep_mulligans, reach_main_phase};
use super::*;
use baylee_core::ids::CardIndex;

fn island() -> CardIndex {
    card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
}
/// `{4}{U}` sorcery — "you *may* waterbend {6}", then draw two, or shuffle
/// the graveyard back and draw seven.
fn spirit_water_revival() -> CardIndex {
    card_index("68979160-b5ce-4787-8a1e-1f40e614c3b0")
}
/// `{1}{W}` 1/1 — a body that may be tapped to help pay.
fn ondu_cleric() -> CardIndex {
    card_index("f4232466-dd6a-49bf-be6c-95905c3ded17")
}

#[track_caller]
fn tap_all_lands(engine: &mut Engine<RegistryLookup>, seat: PlayerId) {
    loop {
        let Pending::Priority { legal, .. } = engine.pending().clone() else {
            panic!("expected priority, got {:?}", engine.pending())
        };
        let Some(&source) = legal.mana_abilities.first() else {
            return;
        };
        engine
            .apply(seat, PlayerAction::ActivateManaAbility { source })
            .expect("a land taps for mana");
    }
}

/// Five Islands, two bodies, and the waterbend sorcery in hand.
fn table() -> Engine<RegistryLookup> {
    let mut engine = Duel::new(4, island())
        .hand(0, &[spirit_water_revival()])
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                island(),
                ondu_cleric(),
                ondu_cleric(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, PlayerId::new(0));
    engine
}

/// Casts the sorcery with all five Islands floating, answering the kicker
/// question with `kick` and the convoke question with nothing. Returns what
/// the cast was refused with, if anything.
#[track_caller]
fn cast_waterbend(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    kick: bool,
) -> Option<EngineError> {
    tap_all_lands(engine, seat);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let card = *legal
        .castable
        .first()
        .expect("five Islands pay {4}{U} with nothing waterbent");
    engine
        .apply(seat, PlayerAction::CastSpell { card })
        .expect("the spell is castable");

    // Two questions, answered by which one arrived — the convoke stage
    // publishes `ChooseTargets` like a targeting stage would, so a fixed
    // script would agree with the engine however wrong it was.
    for _ in 0..8 {
        let answer = match engine.pending() {
            Pending::YesNo { .. } => PlayerAction::YesNo(kick),
            Pending::ChooseTargets { .. } => PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![],
            },
            _ => return None,
        };
        if let Err(err) = engine.apply(seat, answer) {
            return Some(err);
        }
    }
    panic!("the cast never finished asking");
}

fn creatures(engine: &Engine<RegistryLookup>, seat: PlayerId) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine.state().object(*id).is_some_and(|o| {
                o.controller == seat
                    && o.characteristics()
                        .types
                        .contains(baylee_core::types::TypeSet::CREATURE)
            })
        })
        .collect()
}

/// Declining the waterbend cost casts the spell for its printed `{4}{U}`.
///
/// The control: the same board, the same two questions, and nothing refused.
/// Without it the test below could pass on a table where the spell was never
/// castable in the first place.
#[test]
fn declining_the_waterbend_cost_casts_the_spell() {
    let seat = PlayerId::new(0);
    let mut engine = table();
    assert!(
        cast_waterbend(&mut engine, seat, false).is_none(),
        "five Islands could not pay {{4}}{{U}}"
    );
    assert!(
        !engine.state().zones.stack_is_empty(),
        "the spell never reached the stack"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == seat),
        "the caster did not keep priority over their own spell: {:?}",
        engine.pending()
    );
}

/// A waterbend the table cannot pay leaves the caster holding priority.
///
/// This is the "ab hier war das Spiel gar nicht mehr spielbar" half of the
/// report. `{4}{U}` plus a waterbent `{6}` is `{10}{U}`; five Islands and two
/// untapped bodies cannot reach it, and the two bodies are worth nothing at
/// all once the convoke question is answered with none. `finish_cast`
/// refuses, correctly, and tears the wizard down.
///
/// What it then did was resume through `run_until_choice`, and to
/// `priority_round` a holder who is no longer being asked has taken their
/// turn — so the refusal handed the question to the *next seat*. The caster
/// was told their cost could not be paid and lost the rest of their own main
/// phase to a spell that never happened, with five mana floating that
/// emptied at the end of the step.
#[test]
fn a_waterbend_that_cannot_be_paid_gives_the_caster_their_turn_back() {
    let seat = PlayerId::new(0);
    let mut engine = table();
    let refused = cast_waterbend(&mut engine, seat, true);
    assert!(
        matches!(refused, Some(EngineError::IllegalAction(_))),
        "the engine paid {{10}}{{U}} out of five Islands: {refused:?}"
    );

    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == seat),
        "the refused cast passed the caster's priority on: {:?}",
        engine.pending()
    );
    // CR 601.2h reverses the whole casting, so nothing was spent either.
    assert!(
        engine.state().zones.stack_is_empty(),
        "the unpayable spell reached the stack"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "the refused cast spent mana it never paid with"
    );
    for id in creatures(&engine, seat) {
        assert!(
            !engine
                .state()
                .object(id)
                .is_some_and(|o| o.status.contains(crate::object::Status::TAPPED)),
            "a creature was convoked for a cast that was refused"
        );
    }
    // And the seat can act again: the spell is still there to be cast for
    // its printed cost.
    let Pending::Priority { legal, .. } = engine.pending() else {
        unreachable!()
    };
    assert!(
        !legal.castable.is_empty(),
        "the caster was handed a turn with nothing left to do in it"
    );
}

/// The convoke question says what it is, and asks for what is there.
///
/// "Sie wollte von mir 99 Targets." It did: the stage published
/// `Pending::ChooseTargets` with `max: 99`, a sentinel standing in for "as
/// many as you like", over a board holding two. Convoke is not targeting and
/// 99 is not a number on this table; both were read off the screen by the one
/// person who could not check them against the source.
#[test]
fn the_convoke_question_is_named_and_bounded_by_the_board() {
    let seat = PlayerId::new(0);
    let mut engine = table();
    tap_all_lands(&mut engine, seat);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority")
    };
    let card = *legal.castable.first().expect("castable");
    engine
        .apply(seat, PlayerAction::CastSpell { card })
        .expect("castable");
    engine
        .apply(seat, PlayerAction::YesNo(false))
        .expect("the waterbend cost is optional");

    let Pending::ChooseTargets {
        options,
        min,
        max,
        reason,
        ..
    } = engine.pending()
    else {
        panic!(
            "the convoke stage asked something else: {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        *reason,
        crate::choice::TargetPrompt::Convoke,
        "the convoke question arrived indistinguishable from targeting"
    );
    assert_eq!(*min, 0, "convoke is never compulsory");
    assert_eq!(
        (*max as usize, options.len()),
        (2, 2),
        "the bound is a sentinel, not the two bodies on the table"
    );
}
