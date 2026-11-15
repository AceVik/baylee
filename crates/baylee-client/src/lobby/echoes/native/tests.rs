use super::*;

fn signed_in(handle: central::Handle) -> LobbyState {
    let mut state = LobbyState::default();
    central::restored_for_test(&mut state, Vec::new());
    state.central.as_mut().unwrap().handle = Some(handle);
    session(&mut state);
    state
}

#[test]
fn departed_account_reply_and_credentials_cannot_populate_a_new_session() {
    let (first, _first_jobs) = central::Handle::queued_for_test();
    let mut state = signed_in(first);
    let old_inbox = Arc::clone(&state.echoes.worker.inbox);
    let old_connection = Arc::clone(&state.echoes.worker.connection);
    let work = state
        .echoes
        .model
        .read(Uuid::now_v7(), Read::Runs(None))
        .unwrap();
    let (second, _second_jobs) = central::Handle::queued_for_test();
    state.central.as_mut().unwrap().handle = Some(second);
    session(&mut state);
    *old_inbox.lock().unwrap() = Some(Finished {
        work,
        response: Some(api::Response::Runs { runs: Vec::new() }),
        failure: None,
        seat: None,
    });
    assert!(!Arc::ptr_eq(&old_inbox, &state.echoes.worker.inbox));
    assert!(!Arc::ptr_eq(
        &old_connection,
        &state.echoes.worker.connection
    ));
    assert!(state.echoes.worker.inbox.lock().unwrap().is_none());
    assert!(!state.echoes.model.busy());
    assert!(!state.echoes.opened);
}

#[test]
fn a_full_or_departed_worker_releases_the_busy_state_without_automatic_retry() {
    let mut state = signed_in(central::Handle::unavailable_for_test());
    let work = state
        .echoes
        .model
        .read(Uuid::now_v7(), Read::Runs(None))
        .unwrap();
    state.echoes.refresh();
    submit(&mut state, work);
    assert!(!state.echoes.model.busy());
    assert_eq!(state.echoes.model.failure(), Some(Error::Busy));
    assert!(state.echoes.reads.is_empty());
    assert!(state.echoes.playing.is_none());
}

#[test]
fn a_stale_control_cannot_open_a_seat_behind_a_confirmation() {
    let (handle, jobs) = central::Handle::queued_for_test();
    let mut state = signed_in(handle);
    state.echoes.confirmation = Some(Confirmation::Withdrawal);
    for action in [
        Action::Read(Read::Runs(None)),
        Action::GoShop,
        Action::Page(Page::Deck),
        Action::InspectCard(baylee_core::ids::CardIndex(90)),
    ] {
        ask(&mut state, action);
    }
    assert!(matches!(
        state.echoes.confirmation,
        Some(Confirmation::Withdrawal)
    ));
    assert!(!state.echoes.model.busy());
    assert_eq!(state.echoes.page, Page::Shop);
    assert!(state.echoes.preview.is_none());
    assert!(jobs.try_recv().is_err());
    ask(&mut state, Action::Cancel);
    assert!(state.echoes.confirmation.is_none());
}

#[test]
fn tower_context_resumes_its_owned_run_without_another_reservation() {
    use bevy::ecs::system::RunSystemOnce;
    let (handle, jobs) = central::Handle::queued_for_test();
    let mut state = signed_in(handle);
    let run = Uuid::now_v7();
    let game = Uuid::now_v7();
    let event = LobbyEvent::TowerSeated {
        seat: baylee_client_core::lobby::SeatHandover {
            game_id: game.to_string(),
            seat: 0,
            seat_token: "synthetic-only".into(),
            local: false,
        },
        run: *run.as_bytes(),
    };
    seated(&mut state, &event);
    let _ = state.lobby.apply(event);
    assert!(crate::lobby::echoes::current(&state));
    assert_eq!(state.echoes.playing, Some((run, game)));
    let mut app = App::new();
    app.insert_resource(state);
    app.world_mut().run_system_once(returned).unwrap();
    let state = app.world().resource::<LobbyState>();
    assert!(state.echoes.opened);
    assert_eq!(state.hub, Hub::Challenges);
    assert!(state.echoes.playing.is_none());
    assert!(state.echoes.model.busy());
    assert!(
        !state.echoes.model.uncertain(),
        "returning only reads the owned run"
    );
    assert!(jobs.try_recv().is_ok());
}

#[test]
fn tower_context_waits_for_the_first_account_worker_and_only_reads_once() {
    use bevy::ecs::system::RunSystemOnce;
    let run = Uuid::now_v7();
    let game = Uuid::now_v7();
    let mut state = LobbyState::default();
    state.echoes.playing = Some((run, game));
    let mut app = App::new();
    app.insert_resource(state);
    app.insert_resource(Time::<()>::default());
    app.world_mut().run_system_once(returned).unwrap();
    assert_eq!(
        app.world().resource::<LobbyState>().echoes.returning,
        Some(run)
    );
    let (handle, jobs) = central::Handle::queued_for_test();
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        central::restored_for_test(&mut state, Vec::new());
        state.central.as_mut().unwrap().handle = Some(handle);
    }
    app.world_mut().run_system_once(tick).unwrap();
    let state = app.world().resource::<LobbyState>();
    assert!(state.echoes.opened);
    assert!(state.echoes.returning.is_none());
    assert!(state.echoes.model.busy());
    assert!(!state.echoes.model.uncertain());
    assert_eq!(state.hub, Hub::Challenges);
    assert!(jobs.try_recv().is_ok());
    app.world_mut().run_system_once(tick).unwrap();
    assert!(jobs.try_recv().is_err(), "never reserve or read twice");
}

#[test]
fn tower_context_never_crosses_an_account_departure() {
    let run = Uuid::now_v7();
    let game = Uuid::now_v7();
    let mut state = LobbyState::default();
    state.echoes.playing = Some((run, game));
    let (handle, _jobs) = central::Handle::queued_for_test();
    central::restored_for_test(&mut state, Vec::new());
    state.central.as_mut().unwrap().handle = Some(handle);
    session(&mut state);
    assert_eq!(state.echoes.playing, Some((run, game)));
    state.echoes.returning = Some(run);
    state.central.as_mut().unwrap().handle = None;
    session(&mut state);
    assert!(state.echoes.playing.is_none());
    assert!(state.echoes.returning.is_none());
    let (handle, _jobs) = central::Handle::queued_for_test();
    state.central.as_mut().unwrap().handle = Some(handle);
    session(&mut state);
    assert!(state.echoes.playing.is_none());
    assert!(state.echoes.returning.is_none());
}

#[test]
fn tower_context_cancelled_before_account_restore_stays_closed_and_idle() {
    use bevy::ecs::system::RunSystemOnce;
    let mut state = LobbyState::default();
    state.echoes.returning = Some(Uuid::now_v7());
    state.echoes.opened = true;
    ask(&mut state, Action::Back);
    let (handle, jobs) = central::Handle::queued_for_test();
    central::restored_for_test(&mut state, Vec::new());
    state.central.as_mut().unwrap().handle = Some(handle);
    session(&mut state);
    let mut app = App::new();
    app.insert_resource(state);
    app.insert_resource(Time::<()>::default());
    app.world_mut().clear_trackers();
    app.world_mut().run_system_once(tick).unwrap();
    let state = app.world().get_resource_ref::<LobbyState>().unwrap();
    assert!(
        !state.is_changed(),
        "an idle poll must not rebuild the lobby"
    );
    assert!(!state.echoes.opened);
    assert!(state.echoes.returning.is_none());
    assert!(jobs.try_recv().is_err());
}
