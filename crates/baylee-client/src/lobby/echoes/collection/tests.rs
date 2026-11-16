use super::*;
use crate::lobby::{
    Hub, LobbyState, Scrolled, central,
    echoes::{Page, native},
};
use bevy::{ecs::system::RunSystemOnce, prelude::*};

fn app(handle: central::Handle) -> App {
    let mut state = LobbyState::default();
    central::restored_for_test(&mut state, Vec::new());
    state.central.as_mut().unwrap().handle = Some(handle);
    let mut app = App::new();
    app.insert_resource(state)
        .insert_resource(Time::<()>::default())
        .init_resource::<Scrolled>();
    app.world_mut().run_system_once(native::tick).unwrap();
    app
}

#[test]
fn failed_or_corrupt_progress_does_not_replace_confirmed_medals() {
    let mut collection = Collection::default();
    let confirmed = Progress {
        duels: 1000,
        victories: 800,
        clears: 100,
        flawless: 30,
    };
    collection.receive(Ok(confirmed));
    assert_eq!(collection.progress, Some(confirmed));
    collection.receive(Err(baylee_client_core::echoes::Error::Unavailable));
    assert!(collection.failed);
    assert_eq!(collection.progress, Some(confirmed));
    collection.receive(Ok(Progress {
        duels: 0,
        ..confirmed
    }));
    assert!(collection.failed);
    assert_eq!(collection.progress, Some(confirmed));
    collection.receive(Ok(confirmed));
    assert!(!collection.failed);
}

#[test]
fn collection_reads_once_while_visible_and_idle_frames_do_not_rebuild() {
    let (handle, jobs) = central::Handle::queued_for_test();
    let mut app = app(handle);
    app.world_mut().run_system_once(poll).unwrap();
    assert!(jobs.try_recv().is_err(), "hidden collection does not poll");
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.hub = Hub::Challenges;
        state.echoes.opened = true;
        state.echoes.page = Page::Collection;
    }
    app.world_mut().run_system_once(poll).unwrap();
    assert!(jobs.try_recv().is_ok());
    assert!(
        app.world()
            .resource::<LobbyState>()
            .echoes
            .collection
            .pending
    );
    app.world_mut().clear_trackers();
    app.world_mut().run_system_once(poll).unwrap();
    assert!(!app.world().resource_ref::<LobbyState>().is_changed());
    assert!(
        jobs.try_recv().is_err(),
        "only one request may be in flight"
    );
    *app.world()
        .resource::<LobbyState>()
        .echoes
        .collection
        .inbox
        .lock()
        .unwrap() = Some(Ok(Progress::default()));
    app.world_mut().run_system_once(poll).unwrap();
    let state = app.world().resource::<LobbyState>();
    assert_eq!(state.echoes.collection.progress, Some(Progress::default()));
    assert!(!state.echoes.collection.pending);
    app.world_mut().clear_trackers();
    app.world_mut().run_system_once(poll).unwrap();
    assert!(!app.world().resource_ref::<LobbyState>().is_changed());
    assert!(
        jobs.try_recv().is_err(),
        "refresh interval does not allocate idle jobs"
    );
}

#[test]
fn old_account_medals_and_late_mailbox_are_discarded_on_account_switch() {
    let (handle, _jobs) = central::Handle::queued_for_test();
    let mut app = app(handle);
    let old = Arc::clone(&app.world().resource::<LobbyState>().echoes.collection.inbox);
    app.world_mut()
        .resource_mut::<LobbyState>()
        .echoes
        .collection
        .progress = Some(Progress::default());
    let (next, _next_jobs) = central::Handle::queued_for_test();
    app.world_mut()
        .resource_mut::<LobbyState>()
        .central
        .as_mut()
        .unwrap()
        .handle = Some(next);
    app.world_mut().run_system_once(native::tick).unwrap();
    *old.lock().unwrap() = Some(Ok(Progress {
        duels: 99,
        victories: 99,
        clears: 0,
        flawless: 0,
    }));
    app.world_mut().run_system_once(poll).unwrap();
    let collection = &app.world().resource::<LobbyState>().echoes.collection;
    assert!(collection.progress.is_none());
    assert!(!collection.pending);
    assert!(!Arc::ptr_eq(&old, &collection.inbox));
}
