use super::*;
use baylee_account::{RankedDeck, RankedSeat, ranked::Assignment};

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

/// Real Bevy presses through the inline card and native queue worker. Hold the
/// account job in flight so the test can observe the selected input and linearize
/// its two authenticated replies without needing a window or live credentials.
#[test]
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
