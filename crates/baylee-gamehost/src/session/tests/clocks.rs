use super::*;

#[test]
fn the_decision_clock_comes_from_the_house_rules() {
    let mut preset = test_preset();
    preset.house_rules.decision_timeout_secs = 42;
    let session = Session::new(&preset).expect("session builds");
    assert_eq!(session.decision_timeout_secs(), 42);
}

/// A seat that runs out of time is answered legally, and the game moves.
///
/// This is the property that matters: an *illegal* timeout answer would
/// leave the same seat being asked the same question forever, which is
/// the exact failure the clock exists to prevent.
#[test]
fn a_timed_out_seat_is_answered_legally() {
    let mut session = Session::new(&test_preset()).expect("session builds");
    let _ = session.pump();
    let (player, action) = timeout(&session).expect("somebody is being asked something");
    assert_eq!(Some(player), session.awaiting_seat());

    let seq_before = session.seq();
    session
        .act(player, action)
        .expect("the timeout answer is legal");
    assert!(session.seq() > seq_before, "the game moved on");
}

/// The house that answers for a timed-out seat plays for that seat's
/// team. It used to be built with no teams, so every other chair looked
/// like an opponent. The engine never offers a teammate as a defender,
/// but the agent's own reading of the table still counted one as a
/// threat. Here the seat is on 5 life beside a teammate with eight hasty
/// goblins, and the opponents have nothing: a house that fears the swing
/// back from its own partner keeps its attackers home. `stand_in` built
/// its agent with the table's teams all along; the clock's did not.
///
/// Asked of [`Session::house_action`] since #258: the clock itself now
/// attacks with nothing, and the house answers only what has no answer
/// that does nothing. That house is still this one.
#[test]
fn a_clock_answer_does_not_fear_a_teammate() {
    use baylee_core::ids::Defender;
    let goblin = DeckEntry {
        card: baylee_cards::decks::by_name("Raging Goblin").expect("a hasty 1/1"),
        print: PrintRef::new(0),
    };
    let mut preset = teamed_preset([Some(1), Some(2), Some(1), Some(2)]);
    preset.seats[0].starting_battlefield = vec![goblin; 4];
    preset.seats[0].starting_life = Some(5);
    preset.seats[2].starting_battlefield = vec![goblin; 8];
    let mut session = Session::new(&preset).expect("session builds");
    let _ = session.pump();
    let me = PlayerId::new(0);
    for _ in 0..200 {
        let seat = session.awaiting_seat().expect("somebody is asked");
        let action = match session.engine.pending() {
            Pending::ChooseAttackers { .. } if seat == me => break,
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::Priority { .. } => PlayerAction::PassPriority,
            other => panic!("an unexpected question before the first attack: {other:?}"),
        };
        session.act(seat, action).expect("a legal answer");
    }
    assert!(
        matches!(session.engine.pending(), Pending::ChooseAttackers { .. }),
        "the table never reached the first seat's attack"
    );

    let (player, action) = house(&session).expect("the attack is asked");
    assert_eq!(player, me);
    let PlayerAction::DeclareAttackers { attackers } = action else {
        panic!("the clock answered an attack with {action:?}")
    };
    assert_eq!(
        attackers.len(),
        4,
        "the house kept goblins home against its own teammate: {attackers:?}"
    );
    assert!(
        attackers
            .iter()
            .all(|(_, defender)| *defender != Defender::Player(PlayerId::new(2))),
        "{attackers:?}"
    );
}

/// A seat that runs out of time at priority passes, with a spell it
/// could cast in hand and the land to cast it (#258). Before, the house
/// played the seat in full and cast the spell for it, which a player
/// who stepped away finds done, and which no button could say ahead of
/// time. The fixture is checked from the other side too: the house,
/// asked the same question, does act, so a pass here is the clock's
/// choice and not the only thing there was to do.
#[test]
fn a_seat_that_runs_out_of_time_at_priority_passes() {
    let elves_card = baylee_cards::decks::by_name("Llanowar Elves").expect("in the pool");
    let elves = DeckEntry {
        card: elves_card,
        print: PrintRef::new(0),
    };
    let forest = DeckEntry {
        card: baylee_cards::decks::by_name("Forest").expect("in the pool"),
        print: PrintRef::new(0),
    };
    let mut preset = test_preset();
    preset.seats[0].starting_hand = Some(vec![elves]);
    preset.seats[0].starting_battlefield = vec![forest];
    let mut session = Session::new(&preset).expect("session builds");
    let _ = session.pump();
    let me = PlayerId::new(0);
    let my_main = |session: &Session| {
        let turn = &session.engine.state().turn;
        session.awaiting_seat() == Some(me)
            && turn.active == me
            && turn.step == baylee_engine::turn::Step::Main
            && matches!(session.engine.pending(), Pending::Priority { .. })
    };
    for _ in 0..200 {
        if my_main(&session) {
            break;
        }
        let seat = session.awaiting_seat().expect("the game goes on");
        let action = match session.engine.pending() {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::Priority { .. } => PlayerAction::PassPriority,
            other => panic!("an unexpected question before the first main phase: {other:?}"),
        };
        session.act(seat, action).expect("a legal answer");
    }
    assert!(
        my_main(&session),
        "the table never reached the seat's main phase"
    );
    let (_, house) = house(&session).expect("the seat is asked");
    assert_ne!(
        house,
        PlayerAction::PassPriority,
        "the house would pass here too, so this proves nothing"
    );
    assert_eq!(timeout(&session), Some((me, PlayerAction::PassPriority)));

    clock_answers(&mut session, me)
        .expect("the seat is being asked")
        .expect("passing is legal");
    let state = session.engine.state();
    let hand = state
        .zones
        .list(baylee_engine::zone::ZoneLocation::Hand(me));
    assert!(
        hand.iter().any(|&id| state
            .object(id)
            .and_then(|o| o.card)
            .is_some_and(|card| card.index == elves_card)),
        "the Elves are still in hand"
    );
    assert!(state.zones.stack_is_empty(), "nothing was cast");
    // The house's own first step here is tapping the Forest for the
    // Elves, which leaves both of the above true.
    let tapped: Vec<_> = state
        .zones
        .list(baylee_engine::zone::ZoneLocation::Battlefield)
        .iter()
        .filter_map(|&id| state.object(id))
        .filter(|o| o.controller == me && o.status.contains(baylee_engine::object::Status::TAPPED))
        .map(|o| o.id)
        .collect();
    assert!(tapped.is_empty(), "the clock tapped {tapped:?}");
}

/// The clock's order, against an engine that refuses doing nothing: the
/// empty declaration first, and the house's own answer once it is
/// refused. Written against a closure because no card in the pool can
/// make that refusal today (see [`by_clock`]).
#[test]
fn a_refused_answer_that_does_nothing_falls_back_to_the_house() {
    use baylee_core::ids::{Defender, ObjectId};
    let attacker = ObjectId::new(3, 0);
    let pending = Pending::ChooseAttackers {
        player: PlayerId::new(0),
        attackers: vec![attacker],
        defenders: vec![Defender::Player(PlayerId::new(1))],
        required: Vec::new(),
        limits: Vec::new(),
    };
    let nothing = PlayerAction::DeclareAttackers {
        attackers: Vec::new(),
    };
    let forced = PlayerAction::DeclareAttackers {
        attackers: vec![(attacker, Defender::Player(PlayerId::new(1)))],
    };
    // "Attacks each combat if able": the engine takes only the attack.
    let refusing = |tried: &mut Vec<PlayerAction>, action: PlayerAction| {
        tried.push(action.clone());
        if action == nothing {
            Err("the creature attacks if able".to_string())
        } else {
            Ok(())
        }
    };

    let mut tried = Vec::new();
    let out = by_clock(&mut tried, &pending, refusing, |_| Some(forced.clone()));
    assert_eq!(out, Some(Ok(())));
    assert_eq!(tried, vec![nothing.clone(), forced.clone()]);

    // Accepted, the house is never asked.
    let mut tried = Vec::new();
    let out = by_clock(
        &mut tried,
        &pending,
        |tried: &mut Vec<PlayerAction>, action| {
            tried.push(action);
            Ok::<(), String>(())
        },
        |_| panic!("the house was asked although doing nothing was accepted"),
    );
    assert_eq!(out, Some(Ok(())));
    assert_eq!(tried, vec![nothing]);

    // A question with no answer that does nothing goes to the house at once.
    let discard = Pending::DiscardChoice {
        player: PlayerId::new(0),
        count: 1,
    };
    let chosen = PlayerAction::ChooseObjects {
        objects: vec![attacker],
    };
    let mut tried = Vec::new();
    let out = by_clock(
        &mut tried,
        &discard,
        |tried: &mut Vec<PlayerAction>, action| {
            tried.push(action);
            Ok::<(), String>(())
        },
        |_| Some(chosen.clone()),
    );
    assert_eq!(out, Some(Ok(())));
    assert_eq!(tried, vec![chosen]);
}

/// The clock's answer is marked on the seat it answered for, in the very
/// views that answer sends out. An automation setting from the seat
/// afterwards is not an answer and leaves the mark; the seat's own next
/// decision clears it.
#[test]
fn the_clock_marks_the_seat_until_it_answers_itself() {
    let mut session = Session::new(&test_preset()).expect("session builds");
    let _ = session.pump();
    let me = PlayerId::new(0);
    until_asked(&mut session, me);

    let routed = clock_answers(&mut session, me)
        .expect("the seat is being asked")
        .expect("the house's answer is legal");
    let seats = routed_view(&routed, me).seats;
    assert_eq!(seats[0].house_answered, Some(HouseAnswer::Clock));
    assert_eq!(seats[1].house_answered, None, "an AI chair is never marked");

    let turn = session.engine.state().turn.number;
    let routed = session
        .act(
            me,
            PlayerAction::SetPriorityHold(baylee_engine::choice::PriorityHold::UntilEndOfTurn {
                turn,
            }),
        )
        .expect("a seat may state a standing order at any time");
    assert_eq!(
        routed_view(&routed, me).seats[0].house_answered,
        Some(HouseAnswer::Clock),
        "an automation setting is not the seat answering"
    );

    until_asked(&mut session, me);
    let (_, action) = timeout(&session).expect("the seat is asked");
    let routed = session.act(me, action).expect("a legal answer");
    assert_eq!(routed_view(&routed, me).seats[0].house_answered, None);
}

/// A timer that fires after its question has moved on answers nothing:
/// one seat's expired clock never takes another seat's decision.
#[test]
fn the_clock_answers_nothing_for_a_seat_that_is_not_asked() {
    let mut session = Session::new(&test_preset()).expect("session builds");
    let _ = session.pump();
    let me = PlayerId::new(0);
    until_asked(&mut session, me);
    let before = session.decision_seq();
    assert!(clock_answers(&mut session, PlayerId::new(1)).is_none());
    assert_eq!(session.decision_seq(), before);
    assert_eq!(seat_view(&session, me).seats[0].house_answered, None);
}

/// The house standing in for an absent player marks the chair, and the
/// player coming back does not clear it: the last decision is still the
/// house's until the player makes one.
#[test]
fn a_stand_in_marks_the_seat_and_a_reconnect_leaves_the_mark() {
    let mut preset = test_preset();
    preset.seats[1].controller = SeatController::Open;
    let mut session = Session::new(&preset).expect("session builds");
    let _ = session.pump();
    let (me, other) = (PlayerId::new(0), PlayerId::new(1));
    until_asked(&mut session, me);

    assert!(session.stand_in(me));
    let routed = session.pump();
    assert_eq!(
        routed_view(&routed, other).seats[0].house_answered,
        Some(HouseAnswer::StandIn),
        "the table is told the house played that chair"
    );
    assert!(session.hand_back(me));
    assert_eq!(
        seat_view(&session, other).seats[0].house_answered,
        Some(HouseAnswer::StandIn)
    );

    until_asked(&mut session, me);
    let (_, action) = timeout(&session).expect("the seat is asked");
    let routed = session.act(me, action).expect("a legal answer");
    assert_eq!(routed_view(&routed, other).seats[0].house_answered, None);
}

/// An AI chair is the house's to play, driven or not, so the clock
/// answering for a driven one marks nothing: the roster already says it.
#[test]
fn the_clock_does_not_mark_a_driven_ai_chair() {
    let mut session = Session::new(&test_preset()).expect("session builds");
    let driven = PlayerId::new(1);
    assert!(session.take_over(driven));
    let _ = session.pump();
    until_asked(&mut session, driven);
    let routed = clock_answers(&mut session, driven)
        .expect("the seat is being asked")
        .expect("the house's answer is legal");
    assert_eq!(routed_view(&routed, driven).seats[1].house_answered, None);
}

/// Answering by timeout over and over drives the game forward rather than
/// deadlocking on a pending nobody can satisfy.
#[test]
fn repeated_timeouts_keep_the_game_moving() {
    let mut session = Session::new(&test_preset()).expect("session builds");
    let _ = session.pump();
    let mut answered = 0;
    for _ in 0..40 {
        let Some((player, action)) = timeout(&session) else {
            break;
        };
        if session.act(player, action).is_err() {
            break;
        }
        answered += 1;
    }
    assert!(answered > 5, "only {answered} timeouts were answered");
}

// ---- every seat's clock, at every seat (owner, 08.10.2026) ---------------

/// `(seat, ms)` of every clock a view names.
fn clocks_in(view: &baylee_view::PlayerView) -> Vec<(u8, u32)> {
    view.clocks
        .iter()
        .map(|clock| (clock.seat.get(), clock.remaining_ms))
        .collect()
}

/// While both seats decide their opening hands, each seat's view names
/// both clocks, each with its own reading. `decision_remaining_ms` tells a
/// seat only its own then, so without `clocks` neither saw the other's.
/// And a seat that has kept still sees who is thinking, and for how long.
#[test]
fn every_seat_is_told_every_deciding_seats_clock() {
    let (zero, one) = (PlayerId::new(0), PlayerId::new(1));
    let mut session = Session::new(&two_humans()).expect("session builds");
    let _ = session.pump();
    let at_zero = session.asked_at(zero).expect("seat 0 is asked");
    let at_one = session.asked_at(one).expect("seat 1 is asked");
    session.set_decision_remaining(zero, at_zero, Some(9_000));
    session.set_decision_remaining(one, at_one, Some(4_000));
    for seat in [zero, one] {
        assert_eq!(
            clocks_in(&seat_view(&session, seat)),
            [(0, 9_000), (1, 4_000)],
            "seat {} was not told both clocks",
            seat.get()
        );
    }

    let routed = session
        .act(zero, PlayerAction::MulliganKeep)
        .expect("seat 0 keeps");
    let kept = routed_view(&routed, zero);
    assert_eq!(kept.awaiting, None, "seat 0 waits on nobody of its own");
    assert_eq!(
        clocks_in(&kept),
        [(1, 4_000)],
        "a seat that has kept no longer saw the seat still deciding"
    );
}

/// From turn 1 on one seat is asked, and the seat that is not is told its
/// clock in `clocks` as in `decision_remaining_ms`: one number, read in
/// one place.
#[test]
fn the_seat_not_asked_sees_the_asked_seats_clock() {
    let mut session = Session::new(&two_humans()).expect("session builds");
    let _ = session.pump();
    for seat in 0..2 {
        session
            .act(PlayerId::new(seat), PlayerAction::MulliganKeep)
            .expect("keeps");
    }
    let asked = session.awaiting_seat().expect("the game goes on");
    let other = PlayerId::new(1 - asked.get());
    let at = session.asked_at(asked).expect("asked");
    session.set_decision_remaining(asked, at, Some(77_000));
    let view = seat_view(&session, other);
    assert_eq!(clocks_in(&view), [(asked.get(), 77_000)]);
    assert_eq!(view.decision_remaining_ms, Some(77_000));
}

/// No entry where no decision clock runs: for an AI chair, at an untimed
/// table, and for a seat whose clock's owner says none runs (a seat on
/// the stand-in window, a table whose curtain is still down).
#[test]
fn a_seat_on_no_decision_clock_has_no_entry() {
    // The house's chair, seat 1, is never on a clock, whoever looks.
    let mut session = Session::new(&test_preset()).expect("session builds");
    let _ = session.pump();
    for _ in 0..60 {
        let view = seat_view(&session, PlayerId::new(0));
        assert!(
            view.clocks.iter().all(|clock| clock.seat.get() == 0),
            "an AI chair was drawn a clock: {:?}",
            view.clocks
        );
        let Some((player, action)) = timeout(&session) else {
            break;
        };
        session.act(player, action).expect("a legal answer");
    }

    let mut untimed = two_humans();
    untimed.house_rules.decision_timeout_secs = 0;
    let mut session = Session::new(&untimed).expect("session builds");
    let _ = session.pump();
    assert!(seat_view(&session, PlayerId::new(0)).clocks.is_empty());

    let mut session = Session::new(&two_humans()).expect("session builds");
    let _ = session.pump();
    let zero = PlayerId::new(0);
    let at = session.asked_at(zero).expect("asked");
    session.set_decision_remaining(zero, at, None);
    assert_eq!(
        clocks_in(&seat_view(&session, PlayerId::new(1))),
        [(1, 30_000)],
        "a seat its clock's owner put on no clock was drawn one"
    );
}

/// An agent answers from a view with no clock in it, any seat's included:
/// elapsed machine time is not an input to a decision (#87).
#[test]
fn an_agents_view_carries_no_clock() {
    let mut session = Session::new(&two_humans()).expect("session builds");
    let _ = session.pump();
    let zero = PlayerId::new(0);
    let at = session.asked_at(zero).expect("asked");
    session.set_decision_remaining(zero, at, Some(5_000));
    assert!(
        !seat_view(&session, PlayerId::new(1)).clocks.is_empty(),
        "this test needs a clock a socket's view would carry"
    );
    let pending = session
        .engine
        .pending_for(zero)
        .expect("seat 0 is asked")
        .clone();
    let view = session.agent_view(zero, &pending);
    assert!(
        view.clocks.is_empty(),
        "an agent was handed {:?}",
        view.clocks
    );
    assert_eq!(view.decision_remaining_ms, None);
}
