//! Real Bevy controls for account-global tasks, independent of gateway admission.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_client_core::challenges::{Action, api};

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
