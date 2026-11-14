use super::*;
use central_api::challenges::{Challenge, Description, Kind, Period, Words};

fn id(value: u8) -> [u8; 16] {
    let mut id = [value; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    id
}
fn words() -> Words {
    Words {
        en: "Ascent".into(),
        de: "Aufstieg".into(),
    }
}
fn season() -> Season {
    Season {
        id: id(1),
        withdrawn: false,
        theme: Box::new(api::Theme {
            period: Period::containing(Kind::Weekly, 1_794_182_401).unwrap(),
            title: words(),
            floors: std::array::from_fn(|_| words()),
            upgrades: std::array::from_fn(|_| {
                std::array::from_fn(|_| api::Upgrade {
                    title: words(),
                    summary: words(),
                    copies: 2,
                    remove: "Shock".into(),
                    add: "Lightning Bolt".into(),
                })
            }),
        }),
    }
}
fn run() -> Run {
    Run {
        id: id(2),
        season: season(),
        revision: 0,
        progress: api::Progress::new(true),
        listed: false,
        attempt: None,
        encounter: Challenge {
            id: id(3),
            withdrawn: false,
            digest: [3; 32],
            description: Description {
                period: season().theme.period,
                title: words(),
                instructions: words(),
                deck: "4 Shock".into(),
                executor: "test".into(),
                target_turn: 10,
                expected_minutes: 12,
            },
        },
    }
}

#[test]
fn uncertain_mutations_keep_identity_until_a_definitive_outcome() {
    let mut state = State {
        season: Some(season()),
        ..State::default()
    };
    let first = state.act(Action::Play, id(10)).unwrap();
    assert!(!first.play);
    state.refused(central_api::Error::Unavailable);
    assert_eq!(
        state.act(Action::Play, id(11)).unwrap().requests[0],
        first.requests[0]
    );
    state.receive(Response::Run {
        run: Box::new(run()),
    });
    let start = state.act(Action::Play, id(12)).unwrap();
    assert!(start.play);
    assert!(
        matches!(start.requests[0], Request::Start { revision: 0, request_id, .. } if request_id == id(12))
    );
    state.refused(central_api::Error::Conflict);
    assert_ne!(
        state.act(Action::Play, id(13)).unwrap().requests[0],
        start.requests[0]
    );
}

#[test]
fn upgrade_preview_requires_confirmation_and_uses_the_current_revision() {
    let mut run = run();
    run.progress.start().unwrap();
    run.progress
        .finish(
            central_api::challenges::AttemptState::Won,
            9,
            Some(1000),
            true,
        )
        .unwrap();
    run.revision = 2;
    let mut state = State {
        run: Some(run),
        ..State::default()
    };
    assert!(state.act(Action::Upgrade(Track::Resolve), id(4)).is_none());
    assert!(state.run.as_ref().unwrap().progress.route.is_empty());
    let choose = state.act(Action::Choose, id(5)).unwrap();
    assert!(!choose.play);
    assert!(state.act(Action::Upgrade(Track::Assault), id(9)).is_none());
    assert_eq!(
        state.upgrade,
        Some(Track::Resolve),
        "uncertain confirmation cannot silently change its target"
    );
    assert!(matches!(
        choose.requests[0],
        Request::Choose {
            track: Track::Resolve,
            revision: 2,
            ..
        }
    ));
    assert_eq!(
        state.act(Action::Choose, id(6)).unwrap().requests[0],
        choose.requests[0]
    );
}

#[test]
fn rollover_retains_an_active_old_run_and_consent_is_never_implicit() {
    let mut state = State {
        season: Some(season()),
        ..State::default()
    };
    assert!(!state.listed);
    assert!(state.act(Action::Visibility, id(4)).is_none());
    assert!(state.listed);
    let mut next = season();
    next.id = id(8);
    state.receive(Response::Read {
        now: 1_794_182_401 + 604_800,
        season: Some(Box::new(next)),
        standings: Vec::new(),
        run: None,
        active: Some(Box::new(run())),
        badges: Vec::new(),
    });
    assert_eq!(state.visible_season().unwrap().id, id(1));
    assert!(!state.listed);
    assert!(matches!(state.read(), Request::Read { season: Some(season), .. } if season == id(1)));
    assert!(state.act(Action::Abandon, id(5)).is_none());
    assert!(state.abandoning);
    assert!(state.act(Action::Cancel, id(6)).is_none());
    assert!(state.act(Action::Confirm, id(7)).is_none());
}

#[test]
fn owned_game_recovery_selects_its_original_week_without_starting_another_game() {
    let mut state = crate::challenges::State::default();
    let work = state
        .act(crate::challenges::Action::Restore(id(9)), id(10))
        .unwrap();
    assert!(work.restoring && !work.play);
    assert_eq!(work.requests.len(), 1);
    state.receive(
        &work,
        vec![central_api::challenges::Response::Tower {
            response: Box::new(Response::Run {
                run: Box::new(run()),
            }),
        }],
        false,
    );
    assert!(state.tower.opened);
    assert!(state.selected.is_none());
    assert_eq!(state.tower.selected, Some(id(1)));
}
