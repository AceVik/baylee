use super::*;

fn signed_in(handle: central::Handle) -> LobbyState {
    let mut state = LobbyState::default();
    central::restored_for_test(&mut state, Vec::new());
    state.central.as_mut().unwrap().handle = Some(handle);
    session(&mut state);
    state
}

#[test]
fn resting_tower_pages_do_not_rebuild_or_periodically_poll_over_http() {
    use bevy::ecs::system::RunSystemOnce;
    for page in [Page::Shop, Page::Overview, Page::Ranking] {
        let (handle, jobs) = central::Handle::queued_for_test();
        let mut state = signed_in(handle);
        state.hub = Hub::Challenges;
        state.echoes.opened = true;
        state.echoes.loaded = true;
        state.echoes.page = page;
        state.echoes.ranking = Some((Uuid::now_v7(), api::ranking::Lane::Echoes));
        let mut app = App::new();
        app.insert_resource(state)
            .insert_resource(Time::<()>::default())
            .init_resource::<super::super::super::Scrolled>();
        app.world_mut().clear_trackers();
        app.world_mut().run_system_once(tick).unwrap();
        assert!(
            !app.world().resource_ref::<LobbyState>().is_changed(),
            "idle {page:?} must retain its rendered controls and keyboard focus"
        );
        assert!(jobs.try_recv().is_ok(), "one scoped socket acquisition");
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(16));
        app.world_mut().run_system_once(tick).unwrap();
        assert!(jobs.try_recv().is_err(), "no fifteen-second HTTP refresh");
        assert!(!app.world().resource::<LobbyState>().echoes.model.busy());
        assert!(!app.world().resource_ref::<LobbyState>().is_changed());
        app.world_mut().clear_trackers();
        app.world_mut().run_system_once(tick).unwrap();
        assert!(!app.world().resource_ref::<LobbyState>().is_changed());
        assert!(jobs.try_recv().is_err(), "one read may be in flight");
    }
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
    use super::super::domain::v2::catalog::Family;
    let (handle, jobs) = central::Handle::queued_for_test();
    let mut state = signed_in(handle);
    state.echoes.confirmation = Some(Confirmation::Withdrawal);
    for action in [
        Action::Read(Read::Runs(None)),
        Action::GoShop,
        Action::Page(Page::Deck),
        Action::InspectCard(baylee_core::ids::CardIndex(90)),
        Action::Tiers(Family::LifeCoil),
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
    assert!(state.echoes.tiers.is_none());
    assert!(jobs.try_recv().is_err());
    ask(&mut state, Action::Cancel);
    assert!(state.echoes.confirmation.is_none());
    ask(&mut state, Action::Tiers(Family::LifeCoil));
    assert_eq!(state.echoes.tiers, Some(Family::LifeCoil));
    assert!(
        jobs.try_recv().is_err(),
        "tier comparison does not submit work"
    );
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
    app.init_resource::<super::super::Scrolled>();
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
    app.init_resource::<super::super::Scrolled>();
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

#[test]
fn passive_reads_do_not_drop_tab_switches_or_queued_run_reads_and_previews_lock_cleanly() {
    use bevy::ecs::system::RunSystemOnce;
    let (handle, _jobs) = central::Handle::queued_for_test();
    let mut state = signed_in(handle);
    state.hub = Hub::Challenges;
    state.echoes.opened = true;
    let in_flight = state
        .echoes
        .model
        .read(Uuid::now_v7(), Read::Cohorts(None))
        .unwrap();
    assert!(state.echoes.model.busy() && !state.echoes.model.mutating());

    // Header tab navigation and queued run reads remain live during passive reads.
    ask(&mut state, Action::Page(Page::Collection));
    assert_eq!(state.echoes.page, Page::Collection);
    let target_run = Uuid::now_v7();
    ask(&mut state, Action::Read(Read::Run(target_run)));
    assert_eq!(state.echoes.page, Page::Shop);
    assert_eq!(state.echoes.reads.front(), Some(&Read::Run(target_run)));

    // Modal card/starter previews block background actions until closed and clear on navigation.
    ask(
        &mut state,
        Action::InspectCard(baylee_core::ids::CardIndex::new(0)),
    );
    assert!(state.echoes.preview.is_some());
    assert_eq!(
        super::super::keys::stop(Action::ClosePreview, &state.echoes),
        crate::shellkit::focus::Stop::new(super::super::keys::INSPECT.name, "close-preview")
    );
    ask(&mut state, Action::Page(Page::Overview));
    assert_eq!(
        state.echoes.page,
        Page::Shop,
        "card preview sheet blocks background navigation"
    );
    ask(&mut state, Action::ClosePreview);
    assert!(state.echoes.preview.is_none());

    ask(&mut state, Action::Page(Page::Overview));
    ask(&mut state, Action::InspectStarter(Uuid::now_v7(), 0));
    assert!(state.echoes.starter_preview.is_some());
    state.echoes.close();
    assert!(state.echoes.starter_preview.is_none());

    // Finishing the passive read dispatches the queued Read::Run on the next tick.
    state.echoes.opened = true;
    *state.echoes.worker.inbox.lock().unwrap() = Some(Finished {
        work: in_flight,
        response: Some(api::Response::Cohorts {
            consent: 1,
            cohorts: Vec::new(),
        }),
        failure: None,
        seat: None,
    });
    let mut app = App::new();
    app.insert_resource(state)
        .insert_resource(Time::<()>::default())
        .init_resource::<super::super::Scrolled>();
    app.world_mut().run_system_once(tick).unwrap();
    app.world_mut().run_system_once(tick).unwrap();
    let state = app.world().resource::<LobbyState>();
    assert!(state.echoes.reads.is_empty());
    assert!(state.echoes.model.busy());
}

fn with_cohort(state: &mut LobbyState) -> Uuid {
    let content = baylee_tower_content::standard().unwrap();
    let mut identity = baylee_protocol::execution::Identity::native(
        baylee_protocol::execution::Variant::Tower,
        "test",
        "test",
        22,
    );
    identity.rules = "tower-5".into();
    let cohort = api::Cohort {
        id: Uuid::now_v7(),
        contract: super::super::domain::v2::package::Cohort::for_generation(
            identity,
            &content.catalog,
            super::super::domain::v2::generation::RUN,
        )
        .unwrap(),
        catalog: content.catalog,
        starters: content.starters,
        offered: vec![0, 1, 2, 3, 4],
    };
    let id = cohort.id;
    let model = &mut state.echoes.model;
    let work = model.read(Uuid::now_v7(), Read::Cohorts(None)).unwrap();
    model.receive(
        &work,
        Some(api::Response::Cohorts {
            consent: 1,
            cohorts: vec![cohort],
        }),
        None,
    );
    id
}

fn with_run(state: &mut LobbyState) -> api::Run {
    with_cohort(state);
    let cohort = state.echoes.model.cohorts()[0].clone();
    let raw = super::super::domain::v2::run::Run::begin_for(
        &cohort.catalog,
        cohort.starters[0].clone(),
        &[7; 32],
        super::super::domain::v2::generation::RUN,
    )
    .unwrap();
    let run = api::Run::project(Uuid::now_v7(), cohort.id, raw);
    let model = &mut state.echoes.model;
    let work = model.read(Uuid::now_v7(), Read::Run(run.id)).unwrap();
    model.receive(
        &work,
        Some(api::Response::Run {
            run: Box::new(run.clone()),
        }),
        None,
    );
    run
}

#[test]
fn a_shop_press_during_a_read_waits_for_it_and_is_then_sent() {
    use bevy::ecs::system::RunSystemOnce;
    let (handle, jobs) = central::Handle::queued_for_test();
    let mut state = signed_in(handle);
    let run = with_run(&mut state);
    let read = state
        .echoes
        .model
        .read(Uuid::now_v7(), Read::Run(run.id))
        .unwrap();
    assert!(state.echoes.model.busy() && !state.echoes.model.mutating());
    ask(&mut state, Action::Freeze);
    assert!(
        state.echoes.queued.is_some(),
        "the press is kept, not dropped"
    );
    assert!(
        state.echoes.holding(),
        "and the shop shows its controls held"
    );
    assert!(state.echoes.model.receive(
        &read,
        Some(api::Response::Run { run: Box::new(run) }),
        None,
    ));
    let mut app = App::new();
    app.insert_resource(state)
        .insert_resource(Time::<()>::default())
        .init_resource::<super::super::Scrolled>();
    app.world_mut().run_system_once(tick).unwrap();
    let state = app.world().resource::<LobbyState>();
    assert!(state.echoes.queued.is_none());
    assert!(state.echoes.model.mutating(), "the waiting press went out");
    assert!(jobs.try_recv().is_ok());
}

#[test]
fn enrolling_shows_the_echo_explanation_and_sends_consent_only_once_confirmed() {
    let (handle, jobs) = central::Handle::queued_for_test();
    let mut state = signed_in(handle);
    let cohort = with_cohort(&mut state);
    ask(&mut state, Action::Starter(cohort, 1));
    assert!(
        matches!(
            state.echoes.confirmation,
            Some(Confirmation::Enrol {
                starter: 1,
                consent: 1,
                ..
            })
        ),
        "the press asks before it enrols"
    );
    assert!(!state.echoes.model.busy());
    assert!(!state.echoes.model.uncertain());
    assert!(
        jobs.try_recv().is_err(),
        "nothing is sent on the first press"
    );
    ask(&mut state, Action::Cancel);
    assert!(state.echoes.confirmation.is_none());
    assert!(jobs.try_recv().is_err());
    ask(&mut state, Action::Starter(cohort, 1));
    ask(&mut state, Action::Accept);
    assert!(state.echoes.confirmation.is_none());
    assert!(state.echoes.model.uncertain(), "the start request is out");
    assert!(jobs.try_recv().is_ok());
}

/// The duel the shell holds when it ended with a verdict (seat 0 lost):
/// what `ended_as` reads, as `lobby::tests::end_screen` builds it.
fn duel_with_a_verdict() -> crate::Duel {
    use baylee_core::ids::PlayerId;
    let result = baylee_engine::win::GameResult {
        winner: Some(baylee_engine::win::Victor::Player(PlayerId::new(1))),
        reason: baylee_engine::win::EndReason::LastPlayerStanding,
    };
    crate::Duel {
        statics: Some(baylee_view::GameStatic {
            decision_secs: None,
            reconnect_secs: None,
            table_seed: 0,
            view_version: baylee_view::VIEW_VERSION,
            game_id: "g".into(),
            your_seat: PlayerId::new(0),
            seats: vec![baylee_view::SeatIdentity {
                player: PlayerId::new(0),
                display_name: "me".into(),
                is_ai: false,
                away: false,
                team: None,
                guest: false,
            }],
            prints: vec![],
        }),
        interaction: Some(baylee_client_core::Interaction::new(
            baylee_engine::choice::Pending::GameOver(result),
            PlayerId::new(0),
        )),
        ..crate::Duel::default()
    }
}

#[test]
fn a_duel_with_a_verdict_leaves_its_run_settling_and_one_left_unfinished_does_not() {
    // F-TOWER-03 through the real way back: only a duel that ended waits for
    // its verified result; one left running is still the player's to resume.
    use bevy::ecs::system::RunSystemOnce;
    for (duel, settles) in [(Some(duel_with_a_verdict()), true), (None, false)] {
        let run = Uuid::now_v7();
        let mut state = LobbyState::default();
        state.echoes.playing = Some((run, Uuid::now_v7()));
        let mut app = App::new();
        app.insert_resource(state);
        app.insert_resource(Time::<()>::default());
        app.init_resource::<super::super::Scrolled>();
        if let Some(duel) = duel {
            app.insert_resource(duel);
        }
        app.world_mut().run_system_once(returned).unwrap();
        assert_eq!(
            app.world().resource::<LobbyState>().echoes.settling,
            settles.then_some(run)
        );
    }
}
