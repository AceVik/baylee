//! Real Bevy controls for account-global tasks, independent of gateway admission.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_client_core::challenges::{Action, api};
use bevy::ecs::system::RunSystemOnce;
mod tower;

fn screen(lang: Lang) -> (App, std::sync::mpsc::Receiver<central::Job>) {
    let mut app = headless();
    let (handle, jobs) = central::Handle::queued_for_test();
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        central::restored_for_test(&mut state, Vec::new());
        state.central.as_mut().unwrap().handle = Some(handle);
        state.lobby.set_lang(lang);
    }
    app.update();
    settle(&mut app);
    press(
        &mut app,
        crate::lobby::header::nav_press(crate::lobby::header::NavTab::Challenges),
    );
    app.update();
    let id = *uuid::Uuid::now_v7().as_bytes();
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        let model = &mut state.challenges.model;
        model.busy = false;
        model.loaded = true;
        model.now = 1_794_182_401;
        model.selected = Some(id);
        model.detail = Some(api::Challenge {
            id,
            digest: [1; 32],
            withdrawn: false,
            description: api::Description {
                period: api::Period::containing(api::Kind::Daily, model.now).unwrap(),
                title: api::Words {
                    en: "Crossroads".into(),
                    de: "Weggabelung".into(),
                },
                instructions: api::Words {
                    en: "Win this game".into(),
                    de: "Gewinne dieses Spiel".into(),
                },
                deck: "4 Lightning Bolt".into(),
                executor: "test".into(),
                target_turn: 1,
                expected_minutes: 10,
            },
        });
        model.personal = Some(api::TaskProgress {
            challenge: id,
            starts: 0,
            failures: 0,
            hint_level: 0,
            listed: false,
            first_win: None,
            completed_at: None,
            live_week: None,
            tactician: false,
            first_try: false,
            best_turn: None,
        });
    }
    settle(&mut app);
    (app, jobs)
}

#[test]
fn lobby_home_exposes_tasks_and_multiplayer_without_gateway_diagnostics() {
    let (mut app, _jobs) = screen(Lang::De);
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        let task = state.challenges.model.detail.clone().unwrap();
        state.challenges.model.catalogue = vec![task];
        state.challenges.model.badges = api::BadgeKind::ALL
            .into_iter()
            .map(|kind| kind.progress(1))
            .collect();
    }
    press(
        &mut app,
        crate::lobby::header::nav_press(crate::lobby::header::NavTab::Play),
    );
    settle(&mut app);
    let found = presses(&mut app);
    assert!(found.contains(&Press::Hub(HubPress::Tab(Hub::Tables))));
    assert!(
        found
            .iter()
            .any(|p| matches!(p, Press::Challenges(Action::Select(_))))
    );
    assert!(labels(&mut app).contains(&Phrase::ChallengeHome.text(Lang::De).to_string()));
    assert!(!app.world().resource::<LobbyState>().showing_gateway_hub());
    press(
        &mut app,
        Press::Challenges(Action::Badge(api::BadgeKind::Daily)),
    );
    settle(&mut app);
    assert!(labels(&mut app).contains(&Phrase::ChallengeObsidian.text(Lang::De).to_string()));
    assert_eq!(
        app.world().resource::<LobbyState>().challenges.model.badges[0].count,
        1
    );
    press(
        &mut app,
        Press::Challenges(Action::Badge(api::BadgeKind::Daily)),
    );
    settle(&mut app);
    press(&mut app, Press::Hub(HubPress::Tab(Hub::Tables)));
    settle(&mut app);
    assert!(app.world().resource::<LobbyState>().showing_gateway_hub());
    press(
        &mut app,
        crate::lobby::header::nav_press(crate::lobby::header::NavTab::Play),
    );
    settle(&mut app);
    assert!(!app.world().resource::<LobbyState>().showing_gateway_hub());
}

#[test]
fn challenges_work_before_selecting_a_gate_and_default_to_private_scores() {
    for lang in [Lang::En, Lang::De] {
        let (mut app, _jobs) = screen(lang);
        let state = app.world().resource::<LobbyState>();
        assert!(!state.gateway_selected);
        assert_eq!(state.hub, Hub::Challenges);
        assert!(!state.challenges.model.listed);
        let found = presses(&mut app);
        for action in [
            Action::Play,
            Action::Hint,
            Action::Deck,
            Action::Board(api::Board::FirstWin),
            Action::Board(api::Board::BestTurns),
            Action::Board(api::Board::Sprint),
        ] {
            assert!(found.contains(&Press::Challenges(action)), "{found:?}");
        }
        assert!(labels(&mut app).contains(&Phrase::ChallengePrivate.text(lang).to_string()));
        let before = app.world().resource::<UiRebuilds>().total;
        for _ in 0..4 {
            app.update();
        }
        assert_eq!(
            app.world().resource::<UiRebuilds>().total,
            before,
            "idle challenge frames must retain their nodes"
        );
    }
}

#[test]
fn challenge_pages_scroll_over_empty_space_and_keep_the_offset_on_refresh() {
    let (mut app, _jobs) = screen(Lang::De);
    app.world_mut()
        .resource_mut::<LobbyState>()
        .challenges
        .model
        .badges = api::BadgeKind::ALL
        .into_iter()
        .map(|kind| kind.progress(0))
        .collect();
    for tab in [
        crate::lobby::header::NavTab::Challenges,
        crate::lobby::header::NavTab::Play,
    ] {
        press(&mut app, crate::lobby::header::nav_press(tab));
        settle(&mut app);
        let body = app
            .world_mut()
            .query::<(Entity, &Scrollable)>()
            .iter(app.world())
            .find(|(_, list)| list.0 == List::Table)
            .unwrap()
            .0;
        assert!(
            app.world()
                .get::<Pickable>(body)
                .is_none_or(|p| p.is_hoverable),
            "empty space must receive pointer scrolling"
        );
        app.world_mut().entity_mut(body).insert((
            ScrollPosition(Vec2::ZERO),
            ComputedNode {
                size: Vec2::new(1200.0, 400.0),
                content_size: Vec2::new(1200.0, 1600.0),
                ..default()
            },
        ));
        app.world_mut()
            .resource_mut::<Messages<PointerScroll>>()
            .write(aimed(
                body,
                crate::test_pointer::Scroll {
                    unit: MouseScrollUnit::Pixel,
                    x: 0.0,
                    y: -120.0,
                    hit: bevy::picking::backend::HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
                    phase: bevy::input::touch::TouchPhase::Moved,
                },
            ));
        app.update();
        assert!((app.world().get::<ScrollPosition>(body).unwrap().y - 120.0).abs() < 0.01);
        let action = if tab == crate::lobby::header::NavTab::Play {
            Action::Badge(api::BadgeKind::Daily)
        } else {
            Action::Rules
        };
        press(&mut app, Press::Challenges(action));
        settle(&mut app);
        let offset = app
            .world_mut()
            .query::<(&Scrollable, &ScrollPosition)>()
            .iter(app.world())
            .find(|(list, _)| list.0 == List::Table)
            .unwrap()
            .1
            .y;
        assert!(
            (offset - 120.0).abs() < 0.01,
            "reward expansion reset the scroll"
        );
    }
}

#[test]
fn hint_confirmation_blocks_other_actions_and_cancel_never_reveals_a_hint() {
    let (mut app, _jobs) = screen(Lang::De);
    press(&mut app, Press::Challenges(Action::Hint));
    app.update();
    let found = presses(&mut app);
    assert!(found.contains(&Press::Challenges(Action::Confirm)));
    assert!(found.contains(&Press::Challenges(Action::Cancel)));
    assert!(!found.contains(&Press::Challenges(Action::Play)));
    assert!(labels(&mut app).contains(&Phrase::ChallengeHintConfirm.text(Lang::De).to_string()));
    press(&mut app, Press::Challenges(Action::Cancel));
    app.update();
    assert!(
        app.world()
            .resource::<LobbyState>()
            .challenges
            .model
            .hint
            .is_none()
    );
    assert_eq!(
        app.world()
            .resource::<LobbyState>()
            .challenges
            .model
            .personal
            .as_ref()
            .unwrap()
            .hint_level,
        0
    );
}

#[test]
fn changing_central_account_clears_all_private_challenge_state() {
    let (mut app, _jobs) = screen(Lang::En);
    let (handle, _new_jobs) = central::Handle::queued_for_test();
    app.world_mut()
        .resource_mut::<LobbyState>()
        .central
        .as_mut()
        .unwrap()
        .handle = Some(handle);
    app.update();
    let state = app.world().resource::<LobbyState>();
    assert!(state.challenges.model.personal.is_none());
    assert!(state.challenges.model.selected.is_none());
    assert!(state.challenges.model.active.is_empty());
}

#[test]
fn website_challenge_links_wait_for_refresh_and_confirmation_before_being_consumed() {
    use crate::links::Arrived;
    use baylee_client_core::{challenges::Confirmation, deeplink::Link};
    for confirming in [false, true] {
        let mut app = App::new();
        let (handle, _jobs) = central::Handle::queued_for_test();
        let mut state = LobbyState::from_settings(crate::settings::ClientSettings::default());
        central::restored_for_test(&mut state, vec![]);
        state.central.as_mut().unwrap().handle = Some(handle);
        crate::lobby::challenges::ask(&mut state, Action::Refresh);
        state.challenges.model.busy = !confirming;
        state.challenges.model.confirmation = confirming.then_some(Confirmation::Hint(1));
        let id = *uuid::Uuid::now_v7().as_bytes();
        app.insert_resource(state)
            .init_resource::<Mailbox>()
            .init_resource::<Arrived>()
            .add_systems(Update, crate::lobby::links::follow);
        app.world_mut().resource_mut::<Arrived>().link = Some(Link::Challenge(id));
        app.update();
        assert_eq!(
            app.world().resource::<Arrived>().link,
            Some(Link::Challenge(id))
        );
        assert!(
            app.world()
                .resource::<LobbyState>()
                .challenges
                .model
                .selected
                .is_none()
        );
        {
            let mut state = app.world_mut().resource_mut::<LobbyState>();
            state.challenges.model.busy = false;
            state.challenges.model.confirmation = None;
        }
        app.update();
        assert!(app.world().resource::<Arrived>().link.is_none());
        assert_eq!(
            app.world()
                .resource::<LobbyState>()
                .challenges
                .model
                .selected,
            Some(id)
        );
    }
}

#[test]
fn keyboard_walks_challenge_actions_and_confirms_or_cancels_without_a_pointer() {
    use crate::shellkit::focus::{Stop, table};
    let (mut app, _jobs) = screen(Lang::En);
    front_keys::clears_a_despawned_focus(&mut app);
    let controls: Vec<_> = app
        .world_mut()
        .query::<(&Press, Option<&Stop>)>()
        .iter(app.world())
        .filter_map(|(press, stop)| {
            matches!(press, Press::Challenges(_)).then_some((*press, stop.copied()))
        })
        .collect();
    for (press, stop) in controls {
        let stop = stop.unwrap_or_else(|| panic!("unreachable control {press:?}"));
        assert!(table(stop.table).unwrap().stops.contains(&stop.id));
    }
    for _ in 0..20 {
        if front_keys::focused_id(&app) == Some("hint") {
            break;
        }
        front_keys::tab(&mut app, false);
    }
    assert_eq!(front_keys::focused_id(&app), Some("hint"));
    front_keys::press_key(&mut app, KeyCode::Enter, Key::Enter, &[]);
    assert!(
        app.world()
            .resource::<LobbyState>()
            .challenges
            .model
            .confirmation
            .is_some()
    );
    assert_eq!(
        front_keys::focused(&app),
        Some(Stop::new("challenge-confirm", "cancel"))
    );
    front_keys::tab(&mut app, false);
    assert_eq!(front_keys::focused_id(&app), Some("confirm"));
    front_keys::tab(&mut app, false);
    assert_eq!(
        front_keys::focused_id(&app),
        Some("cancel"),
        "the confirmation traps Tab"
    );
    front_keys::press_key(&mut app, KeyCode::Space, Key::Space, &[]);
    assert!(
        app.world()
            .resource::<LobbyState>()
            .challenges
            .model
            .confirmation
            .is_none()
    );
    assert!(
        app.world()
            .resource::<LobbyState>()
            .challenges
            .model
            .hint
            .is_none()
    );
}

#[test]
fn ordinary_seat_response_restores_challenge_context_but_stale_replies_cannot() {
    let (mut app, _jobs) = screen(Lang::En);
    let task = *uuid::Uuid::now_v7().as_bytes();
    let response = answer(
        200,
        &serde_json::json!({
            "game_id": "resumed-challenge", "seat": 0, "seat_token": "synthetic",
            "challenge": task,
        })
        .to_string(),
    );
    let event = crate::lobby::http::decode(Lang::En, Expect::Seat, &response);
    assert!(matches!(event, LobbyEvent::ChallengeSeated { challenge, .. } if challenge == task));
    let epoch = app.world().resource::<LobbyState>().gateway_epoch;
    // This test covers the accepted HTTP reply; the real socket is covered by
    // the gateway integration. Do not open a network host from this headless app.
    app.world_mut().resource_mut::<LobbyState>().connected = true;
    let mailbox = app.world().resource::<Mailbox>().clone();
    mailbox.0.lock().unwrap().push(Reply::Remote(
        epoch.wrapping_add(1),
        Box::new(Reply::Event(event.clone())),
    ));
    app.update();
    assert!(!crate::lobby::challenges::current(
        app.world().resource::<LobbyState>()
    ));
    mailbox
        .0
        .lock()
        .unwrap()
        .push(Reply::Remote(epoch, Box::new(Reply::Event(event))));
    app.update();
    assert!(crate::lobby::challenges::current(
        app.world().resource::<LobbyState>()
    ));
    // The end screen takes the same predicate, and return selects this exact task.
    app.world_mut()
        .run_system_once(crate::lobby::challenges::returned)
        .unwrap();
    let state = app.world().resource::<LobbyState>();
    assert_eq!(state.hub, Hub::Challenges);
    assert_eq!(state.challenges.model.selected, Some(task));
    assert!(state.challenges.model.busy, "the result refresh was queued");
}
