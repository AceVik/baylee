use super::*;

/// Answers every question the way the house would, a player's too, until
/// turn `turn` has begun.
pub(crate) fn play_to_turn(session: &mut Session, turn: u32) {
    for _ in 0..2_000 {
        if session.state().turn.number >= turn {
            return;
        }
        let seat = session.awaiting_seat().expect("somebody is asked");
        let action = session.house_action(seat).expect("the house has an answer");
        session
            .act(seat, action)
            .expect("the house answers legally");
    }
    panic!("turn {turn} never began");
}

/// The owner's usual table is players with house teammates, and a chair
/// that refused every request would make the whole feature dead there.
/// So the house shows its hand to a teammate who asks, at once: showing
/// it changes nothing the house's own agent sees.
#[test]
fn a_request_to_a_chair_the_house_plays_is_accepted_at_once() {
    let mut session = a_kept_table(
        [Some(1), Some(1), Some(2), Some(2)],
        [false, true, false, true],
    );
    let routed = ask(&mut session, 0, 1).expect("a teammate may ask");
    let view = routed_view(&routed, PlayerId::new(0));
    assert_eq!(hands_shown(&view), vec![PlayerId::new(1)], "shown at once");
    assert!(
        view.hand_requested.is_empty(),
        "and nothing is left waiting"
    );
    assert_eq!(
        view.shared_hands[0].cards.len(),
        session
            .state()
            .zones
            .list(ZoneLocation::Hand(PlayerId::new(1)))
            .len(),
        "the whole hand"
    );
}

/// A chair the house plays is shown nobody's hand, so no agent is ever
/// handed one, and it says nothing about itself: it has no socket to say
/// it over.
#[test]
fn a_chair_the_house_plays_is_shown_no_hand_and_sets_nothing() {
    let mut session = a_kept_table(
        [Some(1), Some(1), Some(2), Some(2)],
        [false, true, false, true],
    );
    assert!(
        show_to(&mut session, 0, &[1]).is_err(),
        "no hand for the house"
    );
    assert!(
        seat_view(&session, PlayerId::new(0))
            .hand_shared_with
            .is_empty()
    );
    assert!(
        show_to(&mut session, 1, &[0]).is_err(),
        "an AI chair has no socket to say anything over"
    );
}

/// A chair the house holds for an absent player keeps what the player
/// set, and a request to it waits for the player: the house does not
/// decide about a person's cards, however long they are gone.
#[test]
fn a_request_to_an_absent_player_waits_for_them() {
    let mut session = a_kept_table([Some(1), Some(1), Some(2), Some(2)], [false; 4]);
    assert!(session.stand_in(PlayerId::new(1)));
    ask(&mut session, 0, 1).expect("an absent teammate may be asked");
    play_to_turn(&mut session, 3);
    let view = seat_view(&session, PlayerId::new(0));
    assert_eq!(view.hand_requested, seats(&[1]), "still asked");
    assert!(
        hands_shown(&view).is_empty(),
        "and not answered by the house"
    );
    assert!(session.hand_back(PlayerId::new(1)));
    assert_eq!(
        seat_view(&session, PlayerId::new(1)).hand_requests,
        seats(&[0]),
        "the player finds the request waiting"
    );
    show_to(&mut session, 1, &[0]).expect("and accepts it by showing the hand");
    let view = seat_view(&session, PlayerId::new(0));
    assert_eq!(hands_shown(&view), vec![PlayerId::new(1)]);
    assert!(view.hand_requested.is_empty(), "which answers the request");
}

/// A driven chair is answered over a socket, so it asks, shows and is
/// shown as a player does. Handed back, it is the house's again: shown
/// nothing, and showing its hand to whoever was waiting.
#[test]
fn a_driven_chair_answers_for_itself_until_it_is_handed_back() {
    let mut session = a_kept_table(
        [Some(1), Some(1), Some(2), Some(2)],
        [false, true, false, true],
    );
    assert!(session.take_over(PlayerId::new(1)));
    ask(&mut session, 0, 1).expect("a driven teammate may be asked");
    assert_eq!(
        seat_view(&session, PlayerId::new(0)).hand_requested,
        seats(&[1]),
        "a driven chair decides for itself"
    );
    show_to(&mut session, 0, &[1]).expect("a driven chair may be shown a hand");
    assert_eq!(
        hands_shown(&seat_view(&session, PlayerId::new(1))),
        vec![PlayerId::new(0)]
    );

    assert!(session.release(PlayerId::new(1)));
    let view = seat_view(&session, PlayerId::new(0));
    assert!(
        view.hand_shared_with.is_empty(),
        "the house is shown nothing"
    );
    assert_eq!(
        hands_shown(&view),
        vec![PlayerId::new(1)],
        "and shows its hand to the teammate who was waiting"
    );
    assert!(session.take_over(PlayerId::new(1)));
    assert!(
        hands_shown(&seat_view(&session, PlayerId::new(1))).is_empty(),
        "taken over again, it finds no share left over from before"
    );
}

/// A seat that is turned down may ask again, but not until the next
/// turn: asking again at once would be the same request, louder.
#[test]
fn a_turned_down_request_waits_for_the_next_turn() {
    let mut session = a_kept_table([Some(1), Some(1), Some(2), Some(2)], [false; 4]);
    ask(&mut session, 1, 0).expect("asks");
    session
        .seat_setting(PlayerId::new(0), SeatSetting::DeclineHand(PlayerId::new(1)))
        .expect("turns it down");
    assert!(
        seat_view(&session, PlayerId::new(1))
            .hand_requested
            .is_empty(),
        "the request is answered"
    );
    assert!(ask(&mut session, 1, 0).is_err(), "not again this turn");
    let turn = session.state().turn.number;
    play_to_turn(&mut session, turn + 1);
    ask(&mut session, 1, 0).expect("the next turn it may");
    assert_eq!(
        seat_view(&session, PlayerId::new(0)).hand_requests,
        seats(&[1])
    );
}

/// A setting that changes nothing is answered with nothing: no frame and
/// no new sequence number. A client that repeats its setting costs the
/// table no view.
#[test]
fn a_setting_that_changes_nothing_sends_nothing() {
    let mut session = a_kept_table([Some(1), Some(1), Some(2), Some(2)], [false; 4]);
    assert!(!show_to(&mut session, 0, &[1]).expect("shows").is_empty());
    let seq = session.seq();
    assert!(show_to(&mut session, 0, &[1]).expect("again").is_empty());
    assert!(
        ask(&mut session, 1, 0).expect("asks").is_empty(),
        "asking for a hand already shown"
    );
    assert!(
        session
            .seat_setting(PlayerId::new(0), SeatSetting::DeclineHand(PlayerId::new(1)))
            .expect("declines")
            .is_empty(),
        "declining a request nobody made"
    );
    assert_eq!(session.seq(), seq);
    assert!(!show_to(&mut session, 0, &[]).expect("withdraws").is_empty());
}

/// Showing a hand is not a move in the game. The engine is not asked,
/// so the journal, the snapshot hash and every decision clock's anchor
/// stay exactly where they were.
#[test]
fn a_setting_moves_nothing_in_the_game() {
    let mut session = a_kept_table([Some(1), Some(1), Some(2), Some(2)], [false; 4]);
    let seat = session.awaiting_seat().expect("somebody is asked");
    let hash = session.engine.snapshot_hash();
    let journal = session.state().journal.entries().len();
    let decisions = session.decision_seq();
    let asked = session.asked_at(seat);
    ask(&mut session, 1, 0).expect("asks");
    session
        .seat_setting(PlayerId::new(0), SeatSetting::DeclineHand(PlayerId::new(1)))
        .expect("declines");
    show_to(&mut session, 0, &[1]).expect("shows");
    show_to(&mut session, 0, &[]).expect("withdraws");
    assert_eq!(session.engine.snapshot_hash(), hash);
    assert_eq!(session.state().journal.entries().len(), journal);
    assert_eq!(session.decision_seq(), decisions);
    assert_eq!(session.asked_at(seat), asked);
}

/// A seat on no team has no teammate, so it has nobody to ask or show.
#[test]
fn a_seat_on_no_team_has_nobody_to_ask_or_show() {
    let mut session = a_kept_table([None, None, Some(1), Some(1)], [false; 4]);
    assert!(ask(&mut session, 0, 1).is_err());
    assert!(show_to(&mut session, 0, &[1]).is_err());
    assert!(
        show_to(&mut session, 0, &[])
            .expect("showing nobody is allowed")
            .is_empty(),
        "and changes nothing"
    );
}

#[test]
fn public_hands_survive_private_sharing_and_are_also_available_to_agents() {
    let p0 = PlayerId::new(0);
    let mut preset = teamed_preset([Some(1), Some(1), Some(2), Some(2)]);
    preset.seats[0].capabilities.dev_commands = true;
    preset.seats[0].starting_battlefield.push(DeckEntry {
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
        card: baylee_core::generated::index::REVELATION,
        print: PrintRef::new(0),
    });
    let mut session = a_kept_table_from(preset, [false, false, false, true]);
    show_to(&mut session, 1, &[0]).unwrap();
    for seat in (0..4).map(PlayerId::new) {
        assert_eq!(
            hands_shown(&seat_view(&session, seat)),
            (0..4)
                .map(PlayerId::new)
                .filter(|p| *p != seat)
                .collect::<Vec<_>>()
        );
    }
    let source = session.state().zones.list(ZoneLocation::Battlefield)[0];
    session
        .engine
        .dev_state_mut(p0)
        .unwrap()
        .move_object(
            source,
            ZoneLocation::Graveyard(p0),
            baylee_engine::zone::ZonePosition::Top,
            baylee_engine::event::Cause::Effect,
        )
        .unwrap();
    assert_eq!(
        hands_shown(&seat_view(&session, p0)),
        vec![PlayerId::new(1)]
    );
    assert!(hands_shown(&seat_view(&session, PlayerId::new(3))).is_empty());
}

#[test]
fn share_hand_once_shows_current_snapshot_and_hides_future_draws() {
    let mut session = a_kept_table([Some(1), Some(1), Some(2), Some(2)], [false; 4]);
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let initial_hand = seat_view(&session, p0).hand;
    assert!(!initial_hand.is_empty());

    // Opponent cannot be shown a one-time snapshot.
    assert!(
        session
            .seat_setting(p0, SeatSetting::ShareHandOnce(seats(&[2])))
            .is_err()
    );

    // Teammate asks and is answered with a one-time snapshot.
    ask(&mut session, 1, 0).expect("asks");
    let routed = session
        .seat_setting(p0, SeatSetting::ShareHandOnce(seats(&[1])))
        .expect("shares once");
    let view = routed_view(&routed, p1);
    assert!(view.hand_requested.is_empty(), "request is answered");
    assert_eq!(view.shared_hands.len(), 1);
    assert_eq!(view.shared_hands[0].cards, initial_hand);
    assert!(
        seat_view(&session, p0).hand_shared_with.is_empty(),
        "continuous stream remains off"
    );

    // Advance turns so p0 draws a new card; p1 still sees only the snapshot cards.
    let turn = session.state().turn.number;
    play_to_turn(&mut session, turn + 5);
    let p0_now = seat_view(&session, p0).hand;
    let initial_ids: std::collections::BTreeSet<_> = initial_hand.iter().map(|c| c.id).collect();
    assert!(
        p0_now.iter().any(|c| !initial_ids.contains(&c.id)),
        "p0 drew at least one new card not in the initial snapshot"
    );
    let p1_seen = &seat_view(&session, p1).shared_hands[0].cards;
    assert!(
        p1_seen.iter().all(|c| initial_ids.contains(&c.id)),
        "newly drawn cards remain hidden from a one-time snapshot"
    );

    // Clearing the one-time snapshot hides the hand immediately.
    let cleared = session
        .seat_setting(p0, SeatSetting::ShareHandOnce(SeatSet::new()))
        .expect("clears snapshot");
    assert!(hands_shown(&routed_view(&cleared, p1)).is_empty());
}

#[test]
fn hand_sharing_rule_once_only_and_disabled_are_enforced() {
    let mut once_preset = teamed_preset([Some(1), Some(1), Some(2), Some(2)]);
    once_preset.house_rules.hand_sharing = baylee_core::preset::HandSharingRule::OnceOnly;
    let mut session = a_kept_table_from(once_preset, [false, true, false, false]);
    // Continuous sharing is refused in OnceOnly mode, but ShareHandOnce and RequestHand succeed.
    assert!(show_to(&mut session, 0, &[2]).is_err());
    assert!(
        session
            .seat_setting(PlayerId::new(0), SeatSetting::ShareHand(seats(&[1])))
            .is_err()
    );
    let routed = ask(&mut session, 0, 1).expect("AI teammate answers with snapshot in OnceOnly");
    let view = routed_view(&routed, PlayerId::new(0));
    assert_eq!(hands_shown(&view), vec![PlayerId::new(1)]);

    let mut disabled_preset = teamed_preset([Some(1), Some(1), Some(2), Some(2)]);
    disabled_preset.house_rules.hand_sharing = baylee_core::preset::HandSharingRule::Disabled;
    let mut disabled = a_kept_table_from(disabled_preset, [false; 4]);
    assert!(show_to(&mut disabled, 0, &[1]).is_err());
    assert!(
        disabled
            .seat_setting(PlayerId::new(0), SeatSetting::ShareHandOnce(seats(&[1])))
            .is_err()
    );
    assert!(ask(&mut disabled, 0, 1).is_err());
}

#[test]
fn driven_team_seats_observe_shared_hands_and_preserve_them_in_forks() {
    let mut session = a_kept_table([Some(1), Some(1), Some(2), Some(2)], [true; 4]);
    for s in 0..4 {
        assert!(session.take_over(PlayerId::new(s)));
    }
    session.apply_team_hand_sharing();

    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let p2 = PlayerId::new(2);
    let obs0 = session.observe_view(p0);
    assert_eq!(hands_shown(&obs0), vec![p1]);
    assert_eq!(obs0.shared_hands[0].cards, session.observe_view(p1).hand);

    let fork = session
        .fork_for(p0, baylee_engine::engine::DeckKnowledge::Known, 42)
        .expect("forks with shared teammate hand");
    let fork_obs0 = fork.observe_view(p0);
    assert_eq!(hands_shown(&fork_obs0), vec![p1]);
    assert_eq!(
        fork_obs0.shared_hands[0].cards, obs0.shared_hands[0].cards,
        "teammate's shared hand is preserved in the lookahead fork"
    );
    assert!(
        !fork_obs0.shared_hands.iter().any(|h| h.player == p2),
        "opponent's hand is never shown"
    );
}

/// Under `OnceOnly` a player may not show a teammate their hand for good,
/// only as it stands; and a snapshot reaches that teammate alone: no
/// opponent's view and no spectator's carries it.
#[test]
fn a_snapshot_reaches_the_teammate_alone_and_once_only_refuses_for_good() {
    let mut preset = teamed_preset([Some(1), Some(1), Some(2), Some(2)]);
    preset.house_rules.hand_sharing = baylee_core::preset::HandSharingRule::OnceOnly;
    let mut session = a_kept_table_from(preset, [false; 4]);
    let p0 = PlayerId::new(0);
    assert!(
        session
            .seat_setting(p0, SeatSetting::ShareHand(seats(&[1])))
            .is_err(),
        "a teammate who plays over a socket is refused a hand for good too"
    );
    session
        .seat_setting(p0, SeatSetting::ShareHandOnce(seats(&[1])))
        .expect("shown once");
    assert_eq!(
        hands_shown(&seat_view(&session, PlayerId::new(1))),
        vec![p0]
    );
    for opponent in [2, 3] {
        assert!(
            hands_shown(&seat_view(&session, PlayerId::new(opponent))).is_empty(),
            "seat {opponent} is no teammate"
        );
    }
    assert!(
        session
            .spectator_view()
            .revealed_hands
            .iter()
            .all(|hand| hand.player != p0),
        "a spectator is shown no snapshot"
    );
}
