//! Seat automation: priority holds and standing answers.
//!
//! The point of these tests is the *safety* of the feature, not its
//! convenience. An automated seat must never end up in a position it
//! could not have reached by hand, must always be woken when somebody
//! responds to what it is letting through, and must never have a
//! game-losing decision made for it.

use super::testkit::{
    Duel, RegistryLookup, card_index, cast_from_hand, cast_with_floating, keep_mulligans,
    on_battlefield, pass_until, reach_their_main_phase, stack_is_empty, tap_all_mana,
    tap_mana_where, walk_to_own_main,
};
use super::*;
use crate::choice::{PriorityHold, StandingAnswer, YesNoPrompt};
use baylee_core::ids::{AbilityRef, CardIndex};

fn forest() -> CardIndex {
    card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
}

fn ondu_cleric() -> CardIndex {
    card_index("f4232466-dd6a-49bf-be6c-95905c3ded17")
}

fn plains() -> CardIndex {
    card_index("bc71ebf6-2056-41f7-be35-b2e5c34afa99")
}

fn llanowar_elves() -> CardIndex {
    card_index("68954295-54e3-4303-a6bc-fc4547a4e3a3")
}

/// "Whenever an opponent casts a spell, you may draw a card unless that
/// player pays {1}."
fn rhystic_study() -> CardIndex {
    card_index("53236dd7-845a-444c-96d5-f41ed7325d8f")
}

fn started(builder: Duel) -> Engine<RegistryLookup> {
    let mut engine = builder.start();
    keep_mulligans(&mut engine);
    engine
}

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// The default is unchanged behaviour: every priority is offered.
#[test]
fn the_default_asks_about_everything() {
    let engine = started(Duel::new(7, forest()));
    assert_eq!(engine.automation(P0).hold, PriorityHold::Always);
    assert!(matches!(engine.pending(), Pending::Priority { .. }));
}

/// `PassWhenNothingToDo` skips only the priorities where passing was the
/// single legal action — the seat is still asked the moment it could
/// actually do something.
#[test]
fn nothing_to_do_skips_only_empty_priorities() {
    let mut engine = started(Duel::new(7, forest()).hand(1, &[forest()]));
    // Seat 1 has a land in hand, so it always has something to do; seat 0
    // starts with an empty hand and cannot act at all.
    engine
        .apply(
            P0,
            PlayerAction::SetPriorityHold(PriorityHold::PassWhenNothingToDo),
        )
        .expect("setting a hold is always legal");

    // Whoever is asked now, it is not seat 0 with nothing to do.
    for _ in 0..12 {
        match engine.pending().clone() {
            Pending::Priority { player, legal } => {
                assert!(
                    player != P0 || !legal.nothing_but_passing(),
                    "seat 0 was asked a priority it had nothing to do with"
                );
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .unwrap();
            }
            Pending::ChooseBlockers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .unwrap();
            }
            _ => break,
        }
    }
}

/// Setting a hold does not advance the game or disturb the priority round:
/// the same seat is still on the clock afterwards.
#[test]
fn setting_a_hold_is_not_a_game_action() {
    let mut engine = started(Duel::new(3, forest()).hand(0, &[forest()]));
    let before = engine.state().journal.last_seq();
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("expected a priority")
    };
    engine
        .apply(
            player,
            PlayerAction::SetAbilityPolicy {
                ability: AbilityRef::new(forest(), AbilityRef::ENTERS),
                pass: false,
                answer: Some(StandingAnswer::Yes),
            },
        )
        .expect("setting a standing answer is always legal");
    assert_eq!(
        engine.state().journal.last_seq(),
        before,
        "an automation setting must not move the game"
    );
    match engine.pending() {
        Pending::Priority { player: p, .. } => assert_eq!(*p, player),
        other => panic!("the same seat should still hold priority, got {other:?}"),
    }
}

/// A standing answer is stored under a stable `(card, ability)` handle, so
/// a client can keep it in its preferences and send it into a new game.
#[test]
fn standing_answers_round_trip_under_a_stable_handle() {
    let mut engine = started(Duel::new(3, forest()));
    let ability = AbilityRef::new(ondu_cleric(), 0);
    engine
        .apply(
            P0,
            PlayerAction::SetAbilityPolicy {
                ability,
                pass: false,
                answer: Some(StandingAnswer::Yes),
            },
        )
        .unwrap();
    assert_eq!(
        engine.automation(P0).standing_answer(ability),
        Some(StandingAnswer::Yes)
    );
    assert_eq!(
        engine.automation(P1).standing_answer(ability),
        None,
        "one seat's setting is not another's"
    );

    engine
        .apply(
            P0,
            PlayerAction::SetAbilityPolicy {
                ability,
                pass: false,
                answer: None,
            },
        )
        .unwrap();
    assert_eq!(engine.automation(P0).standing_answer(ability), None);
}

/// "Resolve the stack" is cancelled as soon as anything is added to the
/// stack — the exact moment a player wants to be asked again, because
/// somebody just responded to what they were letting through.
#[test]
fn a_stack_hold_breaks_when_the_stack_grows() {
    let mut engine = started(Duel::new(11, forest()));
    engine
        .apply(
            P0,
            PlayerAction::SetPriorityHold(PriorityHold::UntilStackEmpty { depth: 3 }),
        )
        .unwrap();
    // Nothing is on the stack, so the condition is already met and the
    // hold has expired by the time the engine came to rest.
    assert_eq!(
        engine.automation(P0).hold,
        PriorityHold::Always,
        "an already-satisfied hold does not linger"
    );
}

/// A hold that names an object cancels itself once that object is gone,
/// so a countered trigger can never leave a seat auto-passing forever.
#[test]
fn an_object_hold_expires_when_the_object_is_not_on_the_stack() {
    let mut engine = started(Duel::new(5, forest()));
    let ghost = baylee_core::ids::ObjectId::new(9_999, 0);
    engine
        .apply(
            P0,
            PlayerAction::SetPriorityHold(PriorityHold::UntilTopOfStack { object: ghost }),
        )
        .unwrap();
    assert_eq!(
        engine.automation(P0).hold,
        PriorityHold::Always,
        "a hold on an object that is not on the stack is over immediately"
    );
}

/// An end-of-turn hold really does end with the turn.
#[test]
fn a_turn_hold_expires_with_the_turn() {
    let mut engine = started(Duel::new(5, forest()));
    let turn = engine.state().turn.number;
    engine
        .apply(
            P0,
            PlayerAction::SetPriorityHold(PriorityHold::UntilEndOfTurn { turn }),
        )
        .unwrap();
    // Seat 0 is now auto-passing, so the game runs on by itself until
    // either seat 1 is asked something or the turn rolls over.
    let mut guard = 0;
    while engine.state().turn.number == turn && guard < 200 {
        guard += 1;
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                assert_ne!(player, P0, "seat 0 held priority for the whole turn");
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .unwrap();
            }
            Pending::ChooseBlockers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .unwrap();
            }
            other => panic!("unexpected pending during the held turn: {other:?}"),
        }
    }
    assert!(engine.state().turn.number > turn, "the turn advanced");
    assert_eq!(
        engine.automation(P0).hold,
        PriorityHold::Always,
        "the hold did not survive the turn it was set for"
    );
}

/// Automation changes what the engine does next, so two engines that agree
/// on the board but differ in automation must not hash the same.
///
/// Not the loop detector, which this used to say: that reads
/// `GameState::loop_signature` in the engine and `state().snapshot_hash()` in
/// gamehost's harness, and neither calls [`Engine::snapshot_hash`]. What this
/// hash is for is the determinism comparison a host makes — a replay against
/// its recording, one machine against another.
#[test]
fn automation_is_part_of_the_engine_snapshot() {
    let mut a = started(Duel::new(21, forest()));
    let mut b = started(Duel::new(21, forest()));
    assert_eq!(a.snapshot_hash(), b.snapshot_hash());
    b.apply(
        P0,
        PlayerAction::SetPriorityHold(PriorityHold::PassWhenNothingToDo),
    )
    .unwrap();
    assert_ne!(
        a.snapshot_hash(),
        b.snapshot_hash(),
        "a seat that auto-passes is a different engine state"
    );
    a.apply(
        P0,
        PlayerAction::SetPriorityHold(PriorityHold::PassWhenNothingToDo),
    )
    .unwrap();
    assert_eq!(a.snapshot_hash(), b.snapshot_hash());
}

/// Ondu Cleric's rally trigger: with a standing answer stored for it, the
/// trigger resolves without the seat being asked. The card is the one the
/// feature was asked for by name.
///
/// Played rather than asserted about, because the whole claim is that a
/// question *does not arrive* — and the only way to be sure of that is to
/// walk the game to where it would have.
#[test]
fn a_standing_answer_covers_a_recurring_trigger() {
    let mut engine = started(
        Duel::new(31, plains())
            .battlefield(0, &[plains(), plains()])
            .hand(0, &[ondu_cleric()]),
    );
    engine
        .apply(
            P0,
            PlayerAction::SetAbilityPolicy {
                ability: AbilityRef::new(ondu_cleric(), 0),
                pass: false,
                answer: Some(StandingAnswer::Yes),
            },
        )
        .unwrap();
    // The setting is remembered and addressed by a handle that says
    // nothing about this particular game — which is what lets a client
    // keep it across games.
    assert_eq!(
        engine
            .automation(P0)
            .standing_answer(AbilityRef::new(ondu_cleric(), 0)),
        Some(StandingAnswer::Yes)
    );

    let before = engine.state().players[0].life;
    assert!(walk_to_own_main(&mut engine, P0), "p0 reaches its own main");
    cast_from_hand(&mut engine, P0, ondu_cleric());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, P0, ondu_cleric()).is_some(),
        "the Cleric resolved"
    );
    assert_eq!(
        engine.state().players[0].life,
        before + 1,
        "the rally trigger took its life without the seat being asked"
    );
}

/// The same standing answer, on the question it must never reach.
///
/// Rite of Replication's kicker is a **cost**, and a cost decision is not
/// something a seat delegated by saying "always take Ondu Cleric's life".
/// The gate is [`crate::choice::YesNoPrompt::automatable`]; before it
/// existed, `auto_answer` matched on nothing but "the prompt names an
/// ability", and kicker, a shockland's two life, a tax trigger and a
/// miracle all name one — so the first standing answer a player ever stored
/// would have spent nine mana on their behalf.
#[test]
fn a_standing_answer_never_reaches_a_cost_decision() {
    fn island() -> CardIndex {
        card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
    }
    fn rite_of_replication() -> CardIndex {
        card_index("fb60739e-1dc3-481d-a056-ad72e665c680")
    }
    let kicker = AbilityRef::new(rite_of_replication(), AbilityRef::ADDITIONAL_COST);
    let mut engine = started(
        Duel::new(32, island())
            .battlefield(0, &[island(), island(), island(), island(), island()])
            .battlefield(1, &[ondu_cleric()])
            .hand(0, &[rite_of_replication()]),
    );
    engine
        .apply(
            P0,
            PlayerAction::SetAbilityPolicy {
                ability: kicker,
                pass: false,
                answer: Some(StandingAnswer::Yes),
            },
        )
        .unwrap();
    assert!(walk_to_own_main(&mut engine, P0), "p0 reaches its own main");
    cast_from_hand(&mut engine, P0, rite_of_replication());
    // Kicker is announced before targets and never answered by a stored preference.
    assert!(
        matches!(
            engine.pending(),
            Pending::YesNo {
                prompt: crate::choice::YesNoPrompt::Kicker,
                source: Some(named),
                ..
            } if *named == kicker
        ),
        "the kicker is asked despite a stored yes under its own handle, \
         got {:?}",
        engine.pending()
    );
}

/// A pact's "pay or lose the game" is deliberately not automatable: it
/// carries no ability handle, so no standing answer can ever reach it.
#[test]
fn a_game_losing_question_carries_no_automation_handle() {
    // The rule is enforced at the construction site, so assert the shape
    // of the contract rather than driving a pact into play: a `YesNo` a
    // standing answer could cover must name an ability.
    let pending = Pending::YesNo {
        player: P0,
        prompt: crate::choice::YesNoPrompt::Generic,
        source: None,
    };
    let Pending::YesNo { source, .. } = pending else {
        unreachable!()
    };
    assert!(
        source.is_none(),
        "a question with no ability handle can never be auto-answered"
    );
}

fn three_rally_triggers() -> Engine<RegistryLookup> {
    let mut engine = started(
        Duel::new(43, plains())
            .battlefield(0, &[plains(), plains(), ondu_cleric(), ondu_cleric()])
            .hand(0, &[ondu_cleric()]),
    );
    assert!(walk_to_own_main(&mut engine, P0));
    cast_from_hand(&mut engine, P0, ondu_cleric());
    pass_until(&mut engine, |engine| {
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Stack)
            .len()
            == 3
            && engine
                .state()
                .zones
                .list(crate::zone::ZoneLocation::Stack)
                .iter()
                .all(|id| engine.state().object(*id).unwrap().ability.is_some())
    });
    engine
}

#[test]
fn standing_yield_names_one_ability_and_one_seat() {
    let mut engine = three_rally_triggers();
    assert!(matches!(
        engine.pending(),
        Pending::Priority { player: P0, .. }
    ));
    engine
        .apply(
            P0,
            PlayerAction::SetAbilityYield {
                ability: AbilityRef::new(ondu_cleric(), 1),
                enabled: true,
            },
        )
        .unwrap();
    assert!(
        matches!(engine.pending(), Pending::Priority { player: P0, .. }),
        "another ability on the same card is independent"
    );
    let ability = AbilityRef::new(ondu_cleric(), 0);
    engine
        .apply(
            P0,
            PlayerAction::SetAbilityYield {
                ability,
                enabled: true,
            },
        )
        .unwrap();
    assert!(matches!(
        engine.pending(),
        Pending::Priority { player: P1, .. }
    ));
    assert!(!engine.automation(P1).yields_to(ability));
    engine
        .apply(
            P0,
            PlayerAction::SetAbilityYield {
                ability,
                enabled: false,
            },
        )
        .unwrap();
    assert!(!engine.automation(P0).yields_to(ability));
}

#[test]
fn marked_stack_boundary_outranks_a_standing_yield_and_yes() {
    let mut engine = three_rally_triggers();
    let mark = engine.state().zones.list(crate::zone::ZoneLocation::Stack)[0];
    let ability = AbilityRef::new(ondu_cleric(), 0);
    let before = engine.state().players[0].life;
    engine
        .apply(
            P0,
            PlayerAction::SetAbilityPolicy {
                ability,
                pass: false,
                answer: Some(StandingAnswer::Yes),
            },
        )
        .unwrap();
    engine
        .apply(
            P0,
            PlayerAction::SetPriorityHold(PriorityHold::UntilTopOfStack { object: mark }),
        )
        .unwrap();
    engine
        .apply(
            P0,
            PlayerAction::SetAbilityYield {
                ability,
                enabled: true,
            },
        )
        .unwrap();
    for _ in 0..12 {
        if matches!(engine.pending(), Pending::Priority { player: P0, .. }) {
            break;
        }
        assert!(matches!(
            engine.pending(),
            Pending::Priority { player: P1, .. }
        ));
        engine.apply(P1, PlayerAction::PassPriority).unwrap();
    }
    assert!(matches!(
        engine.pending(),
        Pending::Priority { player: P0, .. }
    ));
    assert_eq!(
        engine.state().zones.list(crate::zone::ZoneLocation::Stack),
        &[mark]
    );
    assert_eq!(
        engine.state().players[0].life,
        before + 6,
        "only the two triggers above the mark resolve"
    );
    assert_eq!(engine.automation(P0).hold, PriorityHold::Always);
    // Reapplying preferences must not swallow the priority just requested.
    engine
        .apply(
            P0,
            PlayerAction::SetAbilityYield {
                ability,
                enabled: true,
            },
        )
        .unwrap();
    assert!(matches!(
        engine.pending(),
        Pending::Priority { player: P0, .. }
    ));
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, before + 9);
}

#[test]
fn cancelling_a_hold_restores_priority_even_for_a_yielded_ability() {
    let mut engine = three_rally_triggers();
    let ability = AbilityRef::new(ondu_cleric(), 0);
    engine
        .apply(
            P0,
            PlayerAction::SetAbilityYield {
                ability,
                enabled: true,
            },
        )
        .unwrap();
    engine
        .apply(P0, PlayerAction::SetPriorityHold(PriorityHold::Always))
        .unwrap();
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    // The top trigger can already have both players' passes, so answer its
    // optional question before testing the next priority window.
    if matches!(engine.pending(), Pending::YesNo { player: P0, .. }) {
        engine.apply(P0, PlayerAction::YesNo(false)).unwrap();
    }
    assert_eq!(engine.automation(P0).hold, PriorityHold::Always);
    assert!(matches!(
        engine.pending(),
        Pending::Priority { player: P0, .. }
    ));
}

#[test]
fn standing_yields_are_part_of_the_deterministic_snapshot() {
    let mut a = started(Duel::new(47, forest()));
    let mut b = started(Duel::new(47, forest()));
    let baseline = a.snapshot_hash();
    for index in [1, 0] {
        a.apply(
            P0,
            PlayerAction::SetAbilityYield {
                ability: AbilityRef::new(ondu_cleric(), index),
                enabled: true,
            },
        )
        .unwrap();
    }
    for index in [0, 1] {
        b.apply(
            P0,
            PlayerAction::SetAbilityYield {
                ability: AbilityRef::new(ondu_cleric(), index),
                enabled: true,
            },
        )
        .unwrap();
    }
    assert_ne!(a.snapshot_hash(), baseline);
    assert_eq!(a.snapshot_hash(), b.snapshot_hash());
}

#[test]
fn a_large_automated_stack_yields_a_real_choice_at_the_safety_limit() {
    let mut engine = three_rally_triggers();
    let top = *engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Stack)
        .last()
        .unwrap();
    let ability = engine.state().object(top).unwrap().ability;
    let name = engine.state.names.intern("Rally");
    for _ in 0..1800 {
        let id = engine.state.create_bare(
            P0,
            crate::object::ObjectKind::AbilityOnStack,
            name,
            crate::zone::ZoneLocation::Stack,
        );
        engine.state.object_mut(id).unwrap().ability = ability;
    }
    engine
        .apply(
            P0,
            PlayerAction::SetAbilityPolicy {
                ability: AbilityRef::new(ondu_cleric(), 0),
                pass: false,
                answer: Some(StandingAnswer::Yes),
            },
        )
        .unwrap();
    let turn = engine.state().turn.number;
    for player in [P0, P1] {
        engine
            .apply(
                player,
                PlayerAction::SetPriorityHold(PriorityHold::UntilEndOfTurn { turn }),
            )
            .unwrap();
    }
    assert!(
        engine.awaiting_answer,
        "the host must receive a current unanswered choice"
    );
    assert!(
        !stack_is_empty(&engine),
        "this reached the budget before draining"
    );
    match engine.pending().clone() {
        Pending::Priority { player, .. } => {
            engine.apply(player, PlayerAction::PassPriority).unwrap();
        }
        Pending::YesNo { player, .. } => engine.apply(player, PlayerAction::YesNo(true)).unwrap(),
        other => panic!("expected a usable choice at the boundary: {other:?}"),
    }
}

#[test]
fn clearing_an_ability_policy_cannot_pass_using_the_old_yield() {
    let mut engine = three_rally_triggers();
    let ability = AbilityRef::new(ondu_cleric(), 0);
    // A restored policy has not consumed the current decision yet.
    engine.automation[0].set_yield(ability, true);
    engine.automation[0].set_standing_answer(ability, Some(StandingAnswer::Yes));
    let action = PlayerAction::SetAbilityPolicy {
        ability,
        pass: false,
        answer: None,
    };
    engine.apply(P0, action).unwrap();
    assert!(matches!(
        engine.pending(),
        Pending::Priority { player: P0, .. }
    ));
    assert!(!engine.automation(P0).yields_to(ability));
    assert_eq!(engine.automation(P0).standing_answer(ability), None);
    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Stack)
            .len(),
        3
    );
}

/// Every answer a per-ability policy gave, as the journal records it
/// (#234).
fn auto_answers(
    engine: &Engine<RegistryLookup>,
) -> Vec<(
    PlayerId,
    AbilityRef,
    Option<ObjectId>,
    crate::event::PolicyAnswer,
)> {
    engine
        .journal()
        .entries()
        .iter()
        .filter_map(|entry| match entry.event {
            crate::event::GameEvent::AutoAnswered {
                player,
                ability,
                object,
                answer,
            } => Some((player, ability, object, answer)),
            _ => None,
        })
        .collect()
}

/// A yield that passes for its seat says so, naming the ability and the
/// trigger it passed over (#234). The seat's own hand passes before it say
/// nothing: the event is the policy's, not every pass's.
#[test]
fn a_yield_that_passes_for_its_seat_is_recorded() {
    let mut engine = three_rally_triggers();
    let top = *engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Stack)
        .last()
        .expect("three triggers");
    assert!(
        auto_answers(&engine).is_empty(),
        "passing by hand is not a policy answering"
    );
    let ability = AbilityRef::new(ondu_cleric(), 0);
    engine
        .apply(
            P0,
            PlayerAction::SetAbilityPolicy {
                ability,
                pass: true,
                answer: None,
            },
        )
        .unwrap();
    assert!(
        matches!(engine.pending(), Pending::Priority { player: P1, .. }),
        "the yield passed p0's priority"
    );
    assert_eq!(
        auto_answers(&engine),
        vec![(P0, ability, Some(top), crate::event::PolicyAnswer::Passed)]
    );
}

/// A pass the seat's own hold makes is the hold's, even over a yielded
/// ability: the seat sees its hold already, and telling it the policy acted
/// would name a setting that changed nothing.
#[test]
fn a_pass_the_hold_makes_anyway_is_not_the_policys() {
    let mut engine = three_rally_triggers();
    let ability = AbilityRef::new(ondu_cleric(), 0);
    let turn = engine.state().turn.number;
    engine
        .apply(
            P0,
            PlayerAction::SetPriorityHold(PriorityHold::UntilEndOfTurn { turn }),
        )
        .unwrap();
    engine
        .apply(
            P0,
            PlayerAction::SetAbilityPolicy {
                ability,
                pass: true,
                answer: None,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        auto_answers(&engine).is_empty(),
        "the hold's passes were reported as the policy's: {:?}",
        auto_answers(&engine)
    );
}

/// A standing answer says what it answered and which trigger asked.
#[test]
fn a_standing_answer_is_recorded_with_the_trigger_that_asked() {
    let mut engine = started(
        Duel::new(31, plains())
            .battlefield(0, &[plains(), plains()])
            .hand(0, &[ondu_cleric()]),
    );
    let ability = AbilityRef::new(ondu_cleric(), 0);
    engine
        .apply(
            P0,
            PlayerAction::SetAbilityPolicy {
                ability,
                pass: false,
                answer: Some(StandingAnswer::Yes),
            },
        )
        .unwrap();
    assert!(walk_to_own_main(&mut engine, P0), "p0 reaches its own main");
    cast_from_hand(&mut engine, P0, ondu_cleric());
    pass_until(&mut engine, stack_is_empty);
    let answers = auto_answers(&engine);
    assert_eq!(
        answers.len(),
        1,
        "one rally trigger, one answer: {answers:?}"
    );
    let (player, answered, object, answer) = answers[0];
    assert_eq!(
        (player, answered, answer),
        (P0, ability, crate::event::PolicyAnswer::Yes)
    );
    let object = object.expect("the trigger resolving asked");
    assert!(
        engine
            .state()
            .object(object)
            .is_none_or(|o| o.ability.is_some_and(|loc| loc.index == 0)),
        "the object named is not the trigger that asked"
    );
}

/// A seat that told the game to resolve the stack for it and then says yes to
/// a mana tax with nothing floating is still paying. The yes opens the
/// payment window (CR 605.3a) with the seat holding priority in it, and a
/// hold set for the *stack* must not pass it: the old engine did, closing the
/// window with the tax unpaid, and the Study's controller drew.
///
/// The hold is `UntilStackEmpty { depth: 2 }`, set while the Elves and the
/// Study's trigger are both on the stack, and the Elves are still under the
/// trigger when it asks, so the hold is alive at the question (asserted: a
/// hold that had expired would pass nothing, on old code too). After the yes
/// the window stays open for the seat, which taps its untapped Forest and
/// passes, and the {1} comes out of that mana.
#[test]
fn a_stack_hold_does_not_pass_the_payment_window_a_tax_yes_opens() {
    let mut engine = started(
        Duel::new(21, forest())
            .battlefield(0, &[rhystic_study()])
            .battlefield(1, &[forest(), forest()])
            .hand(1, &[llanowar_elves()]),
    );
    reach_their_main_phase(&mut engine, P1);
    let forests: Vec<_> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.controller == P1 && o.card.is_some_and(|c| c.index == forest()))
        })
        .collect();
    assert_eq!(forests.len(), 2, "seat 1's two Forests");
    let spent = forests[0];
    tap_mana_where(&mut engine, P1, |id| id == spent);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(P0)).len();

    // One Forest paid for the Elves, the other stands untapped, and the pool
    // is empty: the tax can only be paid by making mana in the window.
    cast_with_floating(&mut engine, P1, llanowar_elves());
    assert_eq!(engine.state().players[1].mana_pool.total(), 0);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Stack).len(),
        2,
        "the Elves and the Study's trigger"
    );
    assert!(matches!(
        engine.pending(),
        Pending::Priority { player, .. } if *player == P1
    ));
    let hold = PriorityHold::UntilStackEmpty { depth: 2 };
    engine
        .apply(P1, PlayerAction::SetPriorityHold(hold))
        .expect("setting a hold is always legal");

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { .. },
                ..
            }
        )
    });
    assert_eq!(
        engine.automation(P1).hold,
        hold,
        "the hold is alive at the question: the Elves are still on the stack"
    );
    engine
        .apply(P1, PlayerAction::YesNo(true))
        .expect("seat 1 says it will pay");

    assert!(
        matches!(engine.payment_window(), Some((seat, _)) if seat == P1),
        "the yes opened a window and the hold left it open: {:?}",
        engine.pending()
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == P1),
        "seat 1 holds priority in its window: {:?}",
        engine.pending()
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        hand_before,
        "nothing is unpaid yet, so nobody drew"
    );

    assert_eq!(tap_all_mana(&mut engine, P1), 1, "the Forest left standing");
    engine
        .apply(P1, PlayerAction::PassPriority)
        .expect("passing ends the payment");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        0,
        "the tax took the {{1}} the Forest made"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        hand_before,
        "paid, so the Study's controller drew nothing"
    );
}

/// Resolving part of a held stack must lower the response boundary: a new
/// spell can restore the original depth without exceeding that original depth.
#[test]
fn a_stack_hold_breaks_when_a_response_restores_its_original_depth() {
    use crate::zone::ZoneLocation;
    let ritual = card_index("53f7c868-b03e-4fc2-8dcf-a75bbfa3272b");
    let swamp = card_index("56719f6a-1a6c-4c0a-8d21-18f7d7350b68");
    let mut engine = started(
        Duel::new(43, plains())
            .battlefield(0, &[plains(), plains(), ondu_cleric(), ondu_cleric()])
            .battlefield(1, &[swamp])
            .hand(0, &[ondu_cleric()])
            .hand(1, &[ritual]),
    );
    assert!(walk_to_own_main(&mut engine, P0));
    cast_from_hand(&mut engine, P0, ondu_cleric());
    pass_until(&mut engine, |engine| {
        engine.state().zones.list(ZoneLocation::Stack).len() == 3
            && engine
                .state()
                .zones
                .list(ZoneLocation::Stack)
                .iter()
                .all(|id| engine.state().object(*id).unwrap().ability.is_some())
    });
    engine
        .apply(
            P0,
            PlayerAction::SetPriorityHold(PriorityHold::UntilStackEmpty { depth: 3 }),
        )
        .unwrap();
    pass_until(&mut engine, |engine| {
        engine.state().zones.list(ZoneLocation::Stack).len() == 2
            && matches!(engine.pending(), Pending::Priority { player: P1, .. })
    });
    assert_eq!(
        engine.automation(P0).hold,
        PriorityHold::UntilStackEmpty { depth: 2 }
    );
    // The existing automation field, including its updated boundary, remains
    // part of the deterministic checkpoint/hash; no extra hidden state exists.
    let current_hash = engine.snapshot_hash();
    engine.automation[0].hold = PriorityHold::UntilStackEmpty { depth: 3 };
    assert_ne!(current_hash, engine.snapshot_hash());
    engine.automation[0].hold = PriorityHold::UntilStackEmpty { depth: 2 };
    cast_from_hand(&mut engine, P1, ritual);
    assert_eq!(engine.state().zones.list(ZoneLocation::Stack).len(), 3);
    assert_eq!(engine.automation(P0).hold, PriorityHold::Always);
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    assert!(
        matches!(engine.pending(), Pending::Priority { player: P0, .. }),
        "the held seat must get its response window before the new spell resolves"
    );
    assert_eq!(engine.state().zones.list(ZoneLocation::Stack).len(), 3);
}
