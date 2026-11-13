use super::*;

fn id(n: u8) -> [u8; 16] {
    let mut id = [n; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    id
}

fn challenge(n: u8) -> Challenge {
    Challenge {
        id: id(n),
        description: api::Description {
            period: api::Period::containing(api::Kind::Daily, 1_794_182_401).unwrap(),
            title: Words {
                en: "A task".into(),
                de: "Eine Aufgabe".into(),
            },
            instructions: Words {
                en: "Win".into(),
                de: "Gewinne".into(),
            },
            deck: "4 Lightning Bolt".into(),
            executor: "test".into(),
            target_turn: 5,
            expected_minutes: 10,
        },
        digest: [1; 32],
        withdrawn: false,
    }
}

fn personal(n: u8) -> TaskProgress {
    TaskProgress {
        challenge: id(n),
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
    }
}

fn selected(n: u8) -> State {
    State {
        selected: Some(id(n)),
        detail: Some(challenge(n)),
        personal: Some(personal(n)),
        ..State::default()
    }
}

fn begin(work: &Work) -> ([u8; 16], bool) {
    match work.requests.first().unwrap() {
        Request::Begin {
            request_id, listed, ..
        } => (*request_id, *listed),
        _ => panic!("expected a reservation"),
    }
}

#[test]
fn rules_are_local_and_reset_when_leaving_the_task() {
    let mut state = selected(1);
    assert!(state.act(Action::Rules, id(8)).is_none());
    assert!(state.show_rules);
    assert!(!state.busy && !state.listed);
    assert!(state.act(Action::Back, id(9)).is_none());
    assert!(!state.show_rules);
}

#[test]
fn ambiguous_start_retries_keep_the_key_even_after_visiting_another_task() {
    let mut state = selected(1);
    let first = state.act(Action::Play, id(10)).unwrap();
    assert_eq!(begin(&first), (id(10), false));
    state.receive(&first, Vec::new(), true);
    let second = state.act(Action::Play, id(11)).unwrap();
    assert_eq!(begin(&second), (id(10), false));
    state.receive(&second, Vec::new(), true);
    let visit = state.act(Action::Select(id(2)), id(12)).unwrap();
    state.receive(
        &visit,
        vec![Response::Detail {
            now: 1_794_182_402,
            challenge: challenge(2),
            standings: Vec::new(),
        }],
        false,
    );
    let other = state.act(Action::Play, id(13)).unwrap();
    assert_eq!(begin(&other).0, id(13));
    state.receive(&other, Vec::new(), true);
    let back = state.act(Action::Select(id(1)), id(14)).unwrap();
    state.receive(
        &back,
        vec![Response::Detail {
            now: 1_794_182_403,
            challenge: challenge(1),
            standings: Vec::new(),
        }],
        false,
    );
    assert_eq!(begin(&state.act(Action::Play, id(15)).unwrap()).0, id(10));
}

#[test]
fn navigation_never_starts_or_discloses_and_cancel_has_no_effect() {
    let mut state = selected(1);
    let visit = state.act(Action::Select(id(2)), id(3)).unwrap();
    assert!(!visit.play);
    assert!(visit.requests.iter().all(|request| matches!(
        request,
        Request::Detail { .. } | Request::Personal { .. } | Request::Progress
    )));
    state.receive(
        &visit,
        vec![Response::Personal {
            progress: personal(2),
        }],
        false,
    );
    assert!(state.act(Action::Hint, id(4)).is_none());
    assert_eq!(state.confirmation, Some(Confirmation::Hint(1)));
    assert!(state.hint.is_none());
    assert!(state.act(Action::Cancel, id(5)).is_none());
    assert!(state.act(Action::Confirm, id(6)).is_none());
    assert_eq!(state.personal.as_ref().unwrap().hint_level, 0);
}

#[test]
fn disclosed_hint_survives_a_later_refresh_failure() {
    let mut state = selected(1);
    state.act(Action::Hint, id(2));
    let work = state.act(Action::Confirm, id(3)).unwrap();
    assert!(matches!(work.requests[0], Request::Hint { level: 1, .. }));
    let hint = Words {
        en: "Consider combat".into(),
        de: "Denke an Kampf".into(),
    };
    state.receive(&work, vec![Response::Hint { text: hint.clone() }], true);
    assert_eq!(state.hint, Some(hint));
    assert!(state.failed);
}

#[test]
fn resume_never_reserves_again_and_abandon_needs_confirmation() {
    let mut state = selected(1);
    let first = state.act(Action::Play, id(2)).unwrap();
    let attempt = Attempt {
        id: id(3),
        challenge: id(1),
        state: api::AttemptState::Playing,
        number: 1,
        host: "https://official.example".into(),
        game: Some(id(4)),
        assisted: false,
        live: true,
        turn: None,
    };
    state.receive(
        &first,
        vec![Response::Attempt {
            attempt: attempt.clone(),
        }],
        true,
    );
    let resumed = state.act(Action::Play, id(5)).unwrap();
    assert!(resumed.requests.is_empty());
    assert_eq!(resumed.resume, Some(attempt));
    state.receive(&resumed, Vec::new(), false);
    assert!(state.act(Action::Abandon, id(6)).is_none());
    let confirmed = state.act(Action::Confirm, id(7)).unwrap();
    assert!(
        matches!(confirmed.requests.first(), Some(Request::Abandon { attempt }) if *attempt == id(3))
    );
}

#[test]
fn late_reply_cannot_overwrite_a_newer_board_and_withdrawal_disables_play() {
    let mut state = selected(1);
    let old = state.act(Action::Board(Board::Sprint), id(2)).unwrap();
    state.receive(&old, Vec::new(), false);
    let current = state.act(Action::Board(Board::BestTurns), id(3)).unwrap();
    state.receive(
        &old,
        vec![Response::Personal {
            progress: personal(2),
        }],
        true,
    );
    assert!(state.busy);
    assert!(!state.failed);
    state.receive(&current, Vec::new(), false);
    state.detail.as_mut().unwrap().withdrawn = true;
    assert!(state.act(Action::Play, id(4)).is_none());
}
