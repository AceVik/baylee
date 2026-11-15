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
    ask(&mut state, Action::Read(Read::Runs(None)));
    assert!(matches!(
        state.echoes.confirmation,
        Some(Confirmation::Withdrawal)
    ));
    assert!(!state.echoes.model.busy());
    assert!(jobs.try_recv().is_err());
    ask(&mut state, Action::Cancel);
    assert!(state.echoes.confirmation.is_none());
}
