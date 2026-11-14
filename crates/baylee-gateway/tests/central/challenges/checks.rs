use super::*;
use baylee_gamehost::record::{Line, Source};

pub(super) fn exercise(
    runtime: &Runtime,
    gateway: &common::Gateway,
    account: &Account,
    task: &api::Challenge,
    candidate: &baylee_train::challenges::Certified,
    agent: &mut Option<tokio::task::JoinHandle<()>>,
    operator: &sea_orm::DatabaseConnection,
) {
    let request = api::Request::Begin {
        challenge: task.id,
        request_id: *Uuid::now_v7().as_bytes(),
        listed: true,
    };
    let api::Response::Attempt { attempt } = account.challenges(request.clone()).unwrap() else {
        panic!("reserved");
    };
    let api::Response::Attempt { attempt: again } = account.challenges(request.clone()).unwrap()
    else {
        panic!("idempotent reserve");
    };
    assert_eq!(again.id, attempt.id);
    let origin = format!("http://127.0.0.1:{}", gateway.port);
    assert_eq!(attempt.host, origin);
    let path = format!("/challenges/{}/play", Uuid::from_bytes(attempt.id));
    let delayed = agent.is_none();
    if delayed {
        assert!(
            matches!(
                account.play_challenge(attempt.id),
                Err(central_api::Error::Conflict)
            ),
            "no certified engine is available yet"
        );
        let api::Response::Attempt { attempt: retained } = account
            .challenges(api::Request::Attempt {
                attempt: attempt.id,
            })
            .unwrap()
        else {
            panic!("retained attempt")
        };
        assert_eq!(retained.state, api::AttemptState::Reserved);
        assert_eq!(
            retained.number, 0,
            "unavailable capacity must not consume a start"
        );
        *agent = Some(runtime.block_on(common::attach_agent(gateway)));
    }
    let admission = account.admit(&origin).unwrap();
    let grant = if delayed {
        race::start_while_recovery_runs(
            runtime,
            operator,
            gateway,
            &admission.token,
            &path,
            attempt.id,
        )
    } else {
        start(gateway, &admission.token, &path)
    };
    assert_eq!(grant["challenge"], serde_json::json!(task.id));
    // The ordinary gateway seat endpoint must retain the same public context
    // after a process restart, independently of the challenge-specific route.
    let grant = start(
        gateway,
        &admission.token,
        &format!("/lobby/games/{}/seat", grant["game_id"].as_str().unwrap()),
    );
    assert_eq!(grant["challenge"], serde_json::json!(task.id));
    let actions: Vec<PlayerAction> = std::str::from_utf8(&candidate.proofs[0].record)
        .unwrap()
        .lines()
        .filter_map(|line| match serde_json::from_str::<Line>(line).unwrap() {
            Line::Input {
                seat: 0,
                by: Source::Seat,
                action,
                ..
            } => Some(action),
            _ => None,
        })
        .collect();
    runtime.block_on(play(
        gateway,
        &admission.token,
        &path,
        &grant,
        &actions,
        candidate.kind == api::Kind::Weekly,
    ));

    completed(account, task, candidate, &attempt, request);
}

fn completed(
    account: &Account,
    task: &api::Challenge,
    candidate: &baylee_train::challenges::Certified,
    attempt: &api::Attempt,
    request: api::Request,
) {
    let completed = wait_finished(account, attempt.id);
    assert_eq!(completed.state, api::AttemptState::Won);
    assert_eq!(completed.number, 1, "resume did not create a second start");
    assert_eq!(completed.turn, Some(candidate.proofs[0].turn));
    let api::Response::Personal { progress } = account
        .challenges(api::Request::Personal { challenge: task.id })
        .unwrap()
    else {
        panic!("personal");
    };
    assert_eq!(progress.starts, 1);
    assert_eq!(progress.failures, 0);
    assert!(progress.first_try && progress.completed_at.is_some());
    for board in [
        api::Board::FirstWin,
        api::Board::BestTurns,
        api::Board::Sprint,
    ] {
        let api::Response::Detail { standings, .. } = account
            .challenges(api::Request::Detail {
                challenge: task.id,
                board,
                offset: 0,
            })
            .unwrap()
        else {
            panic!("ranking");
        };
        if candidate.kind == api::Kind::Weekly && board == api::Board::Sprint {
            assert!(standings.is_empty(), "disconnected victory is not a sprint");
        } else {
            assert_eq!(standings.len(), 1);
            assert!(standings[0].own);
            assert_eq!(standings[0].rank, 1);
        }
    }
    let api::Response::Attempt { attempt: retried } = account.challenges(request).unwrap() else {
        panic!("original reservation");
    };
    assert_eq!(retried.id, completed.id);
    assert_eq!(retried.state, api::AttemptState::Won);
}
