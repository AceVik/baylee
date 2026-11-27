use super::*;
use baylee_account::{RankedDeck, RankedSeat, ranked::Assignment};
use baylee_client_core::aiplayers::Opponents;

fn frozen() -> deck::Frozen {
    deck::Frozen {
        look: baylee_account::advent::Equipment::default(),
        deck: RankedDeck {
            format: Format::Standard,
            cards: vec!["1 Island".into()],
            sideboard: Vec::new(),
            commanders: Vec::new(),
        },
        digest: [1; 32],
    }
}

fn assigned(key: Id) -> QueueState {
    QueueState::Assigned {
        key,
        assignment: Assignment {
            match_id: *Uuid::now_v7().as_bytes(),
            host_id: *Uuid::now_v7().as_bytes(),
            origin: "https://playing.invalid".into(),
            format: Format::Standard,
            account: *Uuid::now_v7().as_bytes(),
            seat: 1,
            deck: [1; 32],
            game: None,
            opponent_ai: None,
        },
    }
}

#[test]
fn queue_keeps_its_operation_and_deck_after_an_ambiguous_reply() {
    let mut w = Worker::default();
    finish(&mut w, 7, Reply::Frozen(7, Ok(frozen())));
    let Some(request @ Request::Join { key, .. }) = w.intent.clone() else {
        panic!("join expected")
    };
    finish(
        &mut w,
        7,
        Reply::Queue(request.clone(), Err(Error::Unavailable)),
    );
    assert_eq!(w.intent.as_ref(), Some(&request));
    assert_eq!(w.frozen.as_ref().unwrap().1.digest, [1; 32]);
    finish(&mut w, 7, Reply::Queue(request, Ok(assigned(key))));
    assert!(w.intent.is_none());
    assert!(!w.needs_deck);
}

#[test]
fn another_devices_queue_with_identical_cards_does_not_wear_this_devices_frozen_look() {
    let mut worker = Worker::default();
    finish(&mut worker, 7, Reply::Frozen(7, Ok(frozen())));
    let Some(Request::Join { key, .. }) = worker.intent.clone() else {
        panic!("expected an admission key")
    };
    worker.observe(assigned(key));
    assert!(worker.frozen.is_some());
    worker.observe(assigned(*Uuid::now_v7().as_bytes()));
    assert!(worker.frozen.is_none() && worker.needs_deck);
}

#[test]
fn a_late_read_from_a_departed_home_does_not_join() {
    let mut w = Worker::default();
    finish(&mut w, 11, Reply::Frozen(10, Ok(frozen())));
    assert_eq!(w.error, Some(Error::Conflict));
    assert!(w.intent.is_none() && w.frozen.is_none());
}

#[test]
fn ending_the_account_detaches_an_in_flight_ranked_status_answer() {
    let (handle, _jobs) = central::Handle::queued_for_test();
    let mut worker = Worker::default();
    worker.session(Some(handle));
    let old: Inbox = Arc::default();
    worker.inbox = Some(old.clone());
    worker.session(None);
    *old.lock().unwrap() = Some(Reply::Queue(
        Request::Status,
        Ok(assigned(*Uuid::now_v7().as_bytes())),
    ));
    assert!(worker.inbox.is_none() && worker.session.is_none());
    assert_eq!(worker.queue, QueueState::Idle);
    assert!(worker.played.is_none());
}

#[test]
fn an_unchanged_queue_does_not_rebuild_and_a_live_game_needs_no_local_deck() {
    let mut w = Worker::default();
    let pending = assigned(*Uuid::now_v7().as_bytes());
    w.observe(pending.clone());
    assert!(w.needs_deck);
    let revision = w.revision;
    w.observe(pending.clone());
    assert_eq!(w.revision, revision);
    let QueueState::Assigned {
        key,
        mut assignment,
    } = pending
    else {
        unreachable!()
    };
    assignment.game = Some(*Uuid::now_v7().as_bytes());
    w.observe(QueueState::Assigned { key, assignment });
    assert!(!w.needs_deck);
}

#[test]
fn only_the_assigned_game_receives_the_one_seat_capability() {
    let QueueState::Assigned { mut assignment, .. } = assigned(*Uuid::now_v7().as_bytes()) else {
        unreachable!()
    };
    let game = Uuid::now_v7();
    assignment.game = Some(*game.as_bytes());
    let play = RankedPlay {
        fence: baylee_account::ranked::replica::Fence {
            match_id: assignment.match_id,
            generation: 1,
        },
        relay_origin: "https://central.invalid".into(),
        assignment,
        renew_at: Instant::now(),
        seat: RankedSeat {
            game_id: game,
            seat: 1,
            seat_token: zeroize::Zeroizing::new("one seat".into()),
        },
    };
    let game_only = ticket(play);
    assert_eq!(game_only.gateway, "https://central.invalid");
    assert_eq!(game_only.game_id, game.to_string());
    assert_eq!(game_only.seat.get(), 1);
    assert_eq!(game_only.seat_token, "one seat");
    assert!(!game_only.watch);
}

#[test]
fn declining_an_assigned_hosts_terms_only_cancels_that_reserved_match() {
    use crate::lobby::{LobbyEvent, Scrolled, front::terms::Subject};
    use baylee_client_core::{i18n::Lang, terms::Ask};
    let mut state = LobbyState::default();
    let home = "https://home.invalid";
    state.gateway = home.into();
    let _ = state.lobby.apply(LobbyEvent::LoggedIn {
        token: "test-home-lobby".into(),
        username: Some("local-home".into()),
    });
    central::enable(&mut state);
    let (handle, _jobs) = central::Handle::queued_for_test();
    state.central.as_mut().unwrap().handle = Some(handle.clone());
    state.ranked_net.session(Some(handle));
    let key = *Uuid::now_v7().as_bytes();
    state.ranked_net.observe(assigned(key));
    state.terms_subject = Subject::Ranked;
    state.terms.ask(Lang::En);
    consent::perform(Ask::SignOut, &mut state, &mut Scrolled::default());
    assert_eq!(state.ranked_net.intent, Some(Request::Cancel { key }));
    assert!(state.central_ready());
    assert_eq!(state.gateway, home);
    assert_eq!(state.lobby.token(), Some("test-home-lobby"));
    assert_eq!(state.terms_subject, Subject::Gateway);
    assert!(!state.terms.up());
}

/// The first search asks whom it accepts, with nothing picked, and freezes
/// nothing until the answer is confirmed: Search answers nothing before an
/// option is picked, every option is offered, and the picked one searches.
fn answer_the_first_search(app: &mut App, choice: Opponents) {
    use crate::lobby::{
        Press, SharedPress,
        confirm::Destructive,
        ranked::Action,
        tests::{press, settle},
    };
    {
        let state = app.world().resource::<LobbyState>();
        assert!(matches!(
            state.confirmation,
            Some(Destructive::RankedOpponents(None))
        ));
        assert!(state.ranked_net.selecting.is_none());
    }
    let search = Press::Shared(SharedPress::ConfirmDestructive);
    let mut presses = app.world_mut().query::<&Press>();
    assert!(
        !presses.iter(app.world()).any(|press| *press == search),
        "Search stays grey, and answers nothing, until a choice is picked"
    );
    for offered in Opponents::ALL {
        let option = Press::Ranked(Action::PickOpponents(offered));
        assert!(presses.iter(app.world()).any(|press| *press == option));
    }
    press(app, Press::Ranked(Action::PickOpponents(choice)));
    settle(app);
    press(app, search);
    settle(app);
}

/// Real Bevy presses through the inline card and native queue worker. Hold the
/// account job in flight so the test can observe the selected input and linearize
/// its two authenticated replies without needing a window or live credentials.
#[test]
#[allow(clippy::too_many_lines)] // one flow of presses, end to end
fn inline_play_presses_freeze_the_chosen_deck_once_and_cancel_the_joined_ticket() {
    use crate::lobby::{
        LobbyEvent, Press,
        ranked::Action,
        tests::{headless, press, settle, stocked},
    };
    use baylee_client_core::lobby::DeckSummary;

    let mut app = headless();
    stocked(&mut app);
    let wanted = Uuid::now_v7();
    let (handle, _jobs) = central::Handle::queued_for_test();
    let epoch = {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        central::enable(&mut state);
        state.gateway_selected = true;
        state.central.as_mut().unwrap().handle = Some(handle.clone());
        let decks = [Uuid::now_v7(), wanted]
            .into_iter()
            .map(|id| {
                handle.set_listed_revision_for_test(id, 3);
                DeckSummary {
                    id: id.to_string(),
                    name: id.to_string(),
                    format: Format::Highlander.as_str().into(),
                    ..Default::default()
                }
            })
            .collect();
        state.lobby.apply(LobbyEvent::Decks(decks));
        state.ranked_net.session(Some(handle));
        state.ranked_net.inbox = Some(Arc::default());
        state.gateway_epoch
    };
    settle(&mut app);
    let tap = |app: &mut App, action| {
        press(app, Press::Ranked(action));
        settle(app);
    };
    tap(&mut app, Action::Open);
    tap(&mut app, Action::Format(Format::Highlander));
    tap(&mut app, Action::ToggleDecks);
    tap(&mut app, Action::Deck(wanted));
    tap(&mut app, Action::Join);
    answer_the_first_search(&mut app, Opponents::Local);
    let ticket = {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        let worker = &mut state.ranked_net;
        let (from, format, selected) = worker.selecting.take().expect("real freeze request");
        assert_eq!(
            (from, format, selected.id()),
            (epoch, Format::Highlander, wanted)
        );
        assert!(worker.intent.is_none());
        let mut answer = frozen();
        answer.deck.format = format;
        // This is the authenticated freeze completion the account job delivers.
        finish(worker, epoch, Reply::Frozen(epoch, Ok(answer)));
        worker.intent.clone().unwrap()
    };
    let Request::Join {
        key,
        format: Format::Highlander,
        deck,
    } = ticket
    else {
        panic!("expected the Highlander queue request for the frozen deck");
    };
    assert_eq!(deck, [1; 32]);
    assert_eq!(
        app.world().resource::<LobbyState>().ranked_net.ticket_ai,
        Some((key, AiOpponents::Local)),
        "the join carries the confirmed choice, retry after retry"
    );
    tap(&mut app, Action::Back);
    tap(&mut app, Action::Open);
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        let worker = &mut state.ranked_net;
        assert_eq!(worker.intent.as_ref(), Some(&ticket));
        assert!(
            worker.selecting.is_none(),
            "returning to the card must not refreeze"
        );
        finish(
            worker,
            epoch,
            Reply::Queue(
                ticket,
                Ok(QueueState::Waiting {
                    key,
                    format: Format::Highlander,
                    queued_at: 3,
                    ai_fill_in_at: None,
                }),
            ),
        );
    }
    settle(&mut app);
    tap(&mut app, Action::Cancel);
    tap(&mut app, Action::Back);
    tap(&mut app, Action::Open);
    let worker = &app.world().resource::<LobbyState>().ranked_net;
    assert_eq!(worker.intent, Some(Request::Cancel { key }));
    assert!(worker.selecting.is_none());
}

fn waiting(key: Id, queued_at: i64, ai_fill_in_at: Option<i64>) -> QueueState {
    QueueState::Waiting {
        key,
        format: Format::Standard,
        queued_at,
        ai_fill_in_at,
    }
}

/// "Nur Menschen" while a search waits: Central keeps a ticket's choice
/// for its key, so the ticket is cancelled and the deck joins again under a
/// new key, while the desk keeps showing the search and its clock.
#[test]
fn a_search_moves_to_another_choice_without_passing_through_idle() {
    let mut state = LobbyState::new();
    let w = &mut state.ranked_net;
    w.ai = AiOpponents::Local;
    finish(w, 7, Reply::Frozen(7, Ok(frozen())));
    let Some(request @ Request::Join { key, .. }) = w.intent.clone() else {
        panic!("join expected")
    };
    let now = w.central_now();
    finish(
        w,
        7,
        Reply::Queue(request, Ok(waiting(key, now, Some(now + 60)))),
    );
    assert_eq!(w.ticket_choice(), Some(AiOpponents::Local));
    assert_eq!(w.searching_since(), Some(now));
    assert!(switch(&mut state, AiOpponents::None, "deck"));
    let w = &mut state.ranked_net;
    assert_eq!(w.intent, Some(Request::Cancel { key }));
    assert!(w.switching());
    assert_eq!(w.fill_in_at(), None, "the countdown goes at once");
    finish(
        w,
        7,
        Reply::Queue(Request::Cancel { key }, Ok(QueueState::Idle)),
    );
    assert!(
        matches!(w.shown_queue(), QueueState::Waiting { .. }),
        "the desk never shows the Idle between the two searches"
    );
    assert_eq!(w.searching_since(), Some(now));
    finish(w, 7, Reply::Frozen(7, Ok(frozen())));
    let Some(request @ Request::Join { key: second, .. }) = w.intent.clone() else {
        panic!("the second join expected")
    };
    assert_ne!(second, key, "a new key, since the old one keeps its choice");
    assert_eq!(w.ticket_ai, Some((second, AiOpponents::None)));
    finish(
        w,
        7,
        Reply::Queue(request, Ok(waiting(second, now + 2, None))),
    );
    assert!(!w.switching());
    assert_eq!(
        w.searching_since(),
        Some(now),
        "the search's clock does not start again"
    );
}

/// A refused cancel ends the move, and the desk shows where the queue
/// really stands; leaving the queue mid-move drops the join that follows.
#[test]
fn a_refused_or_abandoned_move_shows_the_real_queue() {
    let mut state = LobbyState::new();
    let key = *Uuid::now_v7().as_bytes();
    state.ranked_net.queue = waiting(key, 5, Some(65));
    state.ranked_net.ticket_ai = Some((key, AiOpponents::Any));
    assert!(switch(&mut state, AiOpponents::Local, "deck"));
    finish(
        &mut state.ranked_net,
        7,
        Reply::Queue(Request::Cancel { key }, Err(Error::Rejected)),
    );
    assert!(!state.ranked_net.switching());
    assert_eq!(state.ranked_net.error, Some(Error::Rejected));
    state.ranked_net.error = None;
    assert!(switch(&mut state, AiOpponents::Local, "deck"));
    cancel(&mut state);
    assert!(!state.ranked_net.switching());
    assert_eq!(state.ranked_net.intent, Some(Request::Cancel { key }));
}

/// The countdown counts on Central's clock: the join that minted the ticket
/// says what Central's now was, and near the time the worker asks on the
/// second, since fill-in happens inside this account's own poll.
#[test]
fn the_countdown_counts_on_centrals_clock_and_polls_near_its_time() {
    let mut w = Worker::default();
    finish(&mut w, 7, Reply::Frozen(7, Ok(frozen())));
    let Some(request @ Request::Join { key, .. }) = w.intent.clone() else {
        panic!("join expected")
    };
    let local = w.central_now();
    // Central's clock stands 40 seconds behind this one.
    let central = local - 40;
    finish(
        &mut w,
        7,
        Reply::Queue(request, Ok(waiting(key, central, Some(central + 60)))),
    );
    assert!(
        (w.central_now() - central).abs() <= 1,
        "{}",
        w.central_now()
    );
    assert_eq!(
        baylee_client_core::aiplayers::FillIn::at(w.central_now(), w.fill_in_at()),
        baylee_client_core::aiplayers::FillIn::Later
    );
    let soon = w.central_now() + 2;
    finish(
        &mut w,
        7,
        Reply::Queue(Request::Status, Ok(waiting(key, central, Some(soon)))),
    );
    let due = w.next.expect("polls again");
    assert!(due <= Instant::now() + Duration::from_secs(2));
}

/// A search found at launch says nothing of whom it accepts: no kind is
/// shown for it.
#[test]
fn a_ticket_this_client_did_not_send_names_no_kind() {
    let mut w = Worker::default();
    w.observe(waiting(*Uuid::now_v7().as_bytes(), 5, Some(65)));
    assert_eq!(w.ticket_choice(), None);
    assert_eq!(w.fill_in_at(), Some(65), "the countdown still shows");
}

fn assigned_against(
    key: Id,
    opponent_ai: Option<baylee_account::ranked::ai::AiBadge>,
) -> QueueState {
    let QueueState::Assigned { mut assignment, .. } = assigned(key) else {
        unreachable!("assigned() builds an assignment")
    };
    assignment.opponent_ai = opponent_ai;
    QueueState::Assigned { key, assignment }
}

/// A match can come in the poll already under way when the person moves
/// the search, most likely at the countdown's end. An AI they have just
/// turned away is cancelled and the search joins again; a person is played,
/// and the cancel that was to follow is dropped.
#[test]
fn a_match_that_comes_first_is_played_only_if_the_new_choice_takes_it() {
    let house = baylee_account::ranked::ai::AiBadge {
        kind: baylee_account::ranked::ai::AiKind::House,
        label: "Haus-KI · Expert".into(),
    };
    for (opponent, played) in [(Some(house), false), (None, true)] {
        let mut state = LobbyState::new();
        let key = *Uuid::now_v7().as_bytes();
        state.ranked_net.queue = waiting(key, 5, Some(65));
        state.ranked_net.ticket_ai = Some((key, AiOpponents::Local));
        assert!(switch(&mut state, AiOpponents::None, "deck"));
        let w = &mut state.ranked_net;
        finish(
            w,
            7,
            Reply::Queue(Request::Status, Ok(assigned_against(key, opponent))),
        );
        if played {
            assert!(!w.switching());
            assert_eq!(w.intent, None, "the match against a person is played");
        } else {
            assert!(w.switching(), "the search still shows as moving");
            assert_eq!(w.intent, Some(Request::Cancel { key }));
            assert!(matches!(w.shown_queue(), QueueState::Waiting { .. }));
        }
    }
}
