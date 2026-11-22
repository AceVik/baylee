use super::*;
use baylee_account::{RankedDeck, RankedSeat, ranked::Assignment};

fn frozen() -> deck::Frozen {
    deck::Frozen {
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
    assert_eq!(w.frozen.as_ref().unwrap().digest, [1; 32]);
    finish(&mut w, 7, Reply::Queue(request, Ok(assigned(key))));
    assert!(w.intent.is_none());
    assert!(!w.needs_deck);
}

#[test]
fn a_late_read_from_a_departed_home_does_not_join() {
    let mut w = Worker::default();
    finish(&mut w, 11, Reply::Frozen(10, Ok(frozen())));
    assert_eq!(w.error, Some(Error::Conflict));
    assert!(w.intent.is_none() && w.frozen.is_none());
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
        assignment,
        renew_at: Instant::now(),
        seat: RankedSeat {
            game_id: game,
            seat: 1,
            seat_token: zeroize::Zeroizing::new("one seat".into()),
        },
    };
    let game_only = ticket(play);
    assert_eq!(game_only.gateway, "https://playing.invalid");
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
