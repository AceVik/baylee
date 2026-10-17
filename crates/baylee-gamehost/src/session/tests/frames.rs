use super::*;

/// A client that has applied everything is not made to re-render the
/// whole table just because it reconnected.
#[test]
fn a_current_client_gets_nothing_back() {
    let (session, human) = started_session();
    assert!(session.resume(human, session.seq()).is_empty());
}

/// A client that missed everything gets its seat rebuilt — and asking for
/// it does not move the game. Rebuilding through `pump` would have played
/// the AI seats forward as a side effect of someone reconnecting.
#[test]
fn a_stale_client_is_rebuilt_without_advancing_the_game() {
    let (session, human) = started_session();
    let pending_before = format!("{:?}", session.pending());
    let seq_before = session.seq();

    let envelopes = session.resume(human, 0);
    assert!(
        envelopes
            .iter()
            .any(|e| matches!(e.msg, Some(v1::envelope::Msg::StateDelta(_)))),
        "the seat's own view is part of the rebuild"
    );
    assert_eq!(session.seq(), seq_before, "resume moved the sequence");
    assert_eq!(
        format!("{:?}", session.pending()),
        pending_before,
        "resume moved the game"
    );
}

/// The snapshot carries the outstanding choice only for the seat that
/// owes an answer: a spectating reconnect must not be handed someone
/// else's decision.
#[test]
fn only_the_asked_seat_is_sent_the_choice() {
    let (session, human) = started_session();
    let others: Vec<PlayerId> = (0..2).map(PlayerId::new).filter(|p| *p != human).collect();
    let asked = session.pending().asked();
    for seat in others {
        let has_choice = session
            .snapshot(seat)
            .iter()
            .any(|e| matches!(e.msg, Some(v1::envelope::Msg::ChoiceRequest(_))));
        assert_eq!(
            has_choice,
            asked == Some(seat) || matches!(session.pending(), Pending::GameOver(_)),
            "choice went to the wrong seat"
        );
    }
}

/// The clock is the table's, not the engine's.
/// The roster a client is sent: who is at the table, which of them is the
/// house, and the print table without which a `PrintRef` names no card.
#[test]
fn the_opening_payload_describes_the_table() {
    let mut session = Session::new(&test_preset()).expect("session");
    session.describe("g1".to_string(), vec!["Ada".into(), "House AI".into()]);
    let statics = session.game_static(PlayerId::new(0));

    assert_eq!(statics.view_version, baylee_view::VIEW_VERSION);
    assert_eq!(statics.game_id, "g1");
    assert_eq!(statics.your_seat, PlayerId::new(0));
    assert_eq!(statics.seat_name(PlayerId::new(0)), "Ada");
    assert_eq!(statics.seat_name(PlayerId::new(1)), "House AI");
    assert!(!statics.seats[0].is_ai);
    assert!(statics.seats[1].is_ai, "seat 1 of the fixture is the house");
    assert_eq!(statics.prints.len(), 1);
}

/// A seat nobody named still has to be nameable, or the client draws a
/// board with an empty chair opposite.
#[test]
fn an_unnamed_seat_falls_back_to_its_number() {
    let session = Session::new(&test_preset()).expect("session");
    let statics = session.game_static(PlayerId::new(0));
    assert_eq!(statics.seat_name(PlayerId::new(0)), "Seat 0");
    assert_eq!(statics.seat_name(PlayerId::new(1)), "Seat 1");
}

/// The version rides outside the payload so a client can refuse a table it
/// cannot render without first decoding the very structure that changed.
#[test]
fn the_opening_envelope_states_the_view_version_in_the_open() {
    let mut session = Session::new(&test_preset()).expect("session");
    session.describe("g1".to_string(), vec!["Ada".into()]);
    let envelope = session.game_static_envelope(PlayerId::new(0));
    let Some(v1::envelope::Msg::GameStatic(msg)) = envelope.msg else {
        panic!("the opening payload is a GameStatic envelope");
    };
    assert_eq!(msg.view_version, baylee_view::VIEW_VERSION);
    assert_eq!(msg.game_id, "g1");
    let decoded: GameStatic =
        serde_json::from_slice(&msg.static_json).expect("the payload decodes");
    assert_eq!(decoded.your_seat, PlayerId::new(0));
}

/// The card a seat is casting and may take back (`PlayerView::casting`,
/// `PlayerAction::CancelCast`) is in that seat's view only, for as long as
/// the cast is open: a Lightning Bolt at its target question names the Bolt
/// to its caster and nothing to the other seat, whose view would otherwise
/// name a card in a hand it cannot see.
#[test]
fn the_card_being_cast_is_named_to_its_caster_alone() {
    let bolt_card = baylee_cards::by_oracle_id("4457ed35-7c10-48c8-9776-456485fdf070")
        .unwrap()
        .index;
    let mountain = baylee_cards::by_oracle_id("a3fb7228-e76b-4e96-a40e-20b5fed75685")
        .unwrap()
        .index;
    let entry = |card| DeckEntry {
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
        card,
        print: PrintRef::new(0),
    };
    let mut preset = two_humans();
    preset.seats[0].starting_hand = Some(vec![entry(bolt_card)]);
    preset.seats[0].starting_battlefield = vec![entry(mountain)];
    let mut session = Session::new(&preset).expect("session");
    session.pump();
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    for _ in 0..60 {
        if let Pending::Priority { player, legal } = session.pending().clone()
            && player == me
            && !legal.mana_abilities.is_empty()
        {
            break;
        }
        let (player, action) = timeout(&session).expect("a question is out");
        session.act(player, action).expect("a legal answer");
    }
    let Pending::Priority { legal, .. } = session.pending().clone() else {
        panic!(
            "seat 0 holds priority with its Mountain: {:?}",
            session.pending()
        )
    };
    let land = legal.mana_abilities[0];
    session
        .act(me, PlayerAction::ActivateManaAbility { source: land })
        .expect("the Mountain");
    let bolt = session.engine.state().zones.list(ZoneLocation::Hand(me))[0];
    session
        .act(me, PlayerAction::CastSpell { card: bolt })
        .expect("the Bolt");
    assert!(matches!(session.pending(), Pending::ChooseTargets { .. }));
    assert_eq!(seat_view(&session, me).casting, Some(bolt));
    assert_eq!(seat_view(&session, them).casting, None);

    session
        .act(me, PlayerAction::CancelCast)
        .expect("taken back");
    assert_eq!(seat_view(&session, me).casting, None);
    assert_eq!(
        session.engine.state().zones.list(ZoneLocation::Hand(me)),
        &vec![bolt]
    );
}

/// When `pump_at_most(N)` finishes the house's work on the exact `N`-th step,
/// it must emit the waiting human seat's view and question immediately rather
/// than returning an empty batch with `needs_pump() == false`.
#[test]
fn pump_at_most_emits_envelopes_when_the_exact_budget_step_reaches_a_human() {
    let mut probe = Session::new(&test_preset()).expect("probe session");
    let mut exact_steps = 0usize;
    while probe.needs_pump() {
        exact_steps += 1;
        let _ = probe.pump_at_most(1);
    }
    assert!(exact_steps >= 1, "house owes at least one opening step");

    let mut session = Session::new(&test_preset()).expect("session");
    let human = session.human_seats()[0];
    assert!(
        session.needs_pump(),
        "seat 1 is the house and owes its opening mulligan decisions"
    );
    assert!(
        session.pump_at_most(0).is_empty(),
        "a zero budget does no house work and sends no premature frame"
    );
    if exact_steps > 1 {
        let mut partial = Session::new(&test_preset()).expect("partial session");
        assert!(
            partial.pump_at_most(exact_steps - 1).is_empty(),
            "exhausting the budget while the house still owes a step returns no envelopes"
        );
        assert!(partial.needs_pump());
    }

    // Running with a budget equal to `exact_steps` finishes the house's work
    // on the exact budget boundary (`answered == answers`) and must emit the
    // human seat's frames immediately.
    let routed = session.pump_at_most(exact_steps);
    assert!(
        !session.needs_pump(),
        "after {exact_steps} house steps, only the human seat is deciding"
    );
    assert!(
        !routed.is_empty(),
        "completing the house's work on the budget boundary must emit the human seat's frames"
    );
    assert!(
        routed
            .iter()
            .any(|(s, e)| *s == human && matches!(e.msg, Some(v1::envelope::Msg::StateDelta(_)))),
        "human seat must receive its StateDelta"
    );
    assert_eq!(
        asked(&routed)
            .into_iter()
            .map(|(s, _)| s)
            .collect::<Vec<_>>(),
        [human.get()],
        "human seat must receive its ChoiceRequest"
    );
}

/// Under heavy house turns, `act_slice` and `pump_slice` stream intermediate
/// `StateDelta` updates (and spectator views) after each slice without
/// sending premature `ChoiceRequest`s, and reach the exact same state and
/// log sequence as an un-sliced `act`.
#[test]
fn pump_slice_streams_incremental_views_without_premature_questions_and_matches_full_pump() {
    let mut reference = Session::new(&test_preset()).expect("reference session");
    let mut sliced = Session::new(&test_preset()).expect("sliced session");
    sliced.set_spectators(1);
    let human = reference.human_seats()[0];

    reference.pump();
    sliced.pump();
    reference
        .act(human, PlayerAction::MulliganKeep)
        .expect("keep");
    sliced.act(human, PlayerAction::MulliganKeep).expect("keep");

    // Advance until passing priority hands a multi-step turn to the house AI.
    let mut multi_slice_seen = false;
    for _ in 0..40 {
        let Some((seat, action)) = house(&reference) else {
            break;
        };
        assert_eq!(seat, human);
        let ref_out = reference.act(human, action.clone()).expect("reference act");

        let mut slice_out = sliced.act_slice(human, action, 1).expect("sliced act");
        let mut slices = 1usize;
        let mut last_seq = 0u64;
        while sliced.needs_pump() {
            multi_slice_seen = true;
            assert!(
                asked(&slice_out).is_empty(),
                "intermediate slice must not ask a human question while the house is still pumping"
            );
            let view = routed_view(&slice_out, human);
            assert!(
                view.seq > last_seq,
                "each intermediate slice advances the view sequence"
            );
            last_seq = view.seq;
            assert!(
                !sliced.spectator_envelopes().is_empty(),
                "spectators receive live progress deltas during sliced pumping"
            );
            slice_out = sliced.pump_slice(1);
            slices += 1;
            assert!(slices < 500, "house turn must terminate");
        }

        assert_eq!(
            format!("{:?}", asked(&slice_out)),
            format!("{:?}", asked(&ref_out)),
            "final slice asks the exact same question as a full pump"
        );
        assert_eq!(
            sliced.snapshot_hash(),
            reference.snapshot_hash(),
            "sliced pumping is bit-identical to full pumping"
        );
        if multi_slice_seen {
            break;
        }
    }
    assert!(
        multi_slice_seen,
        "fixture must exercise a multi-step house turn across multiple slices"
    );
}
