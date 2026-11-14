//! Optional retained-proof expedition integration, avoiding repeated expensive generation.
use super::*;
use baylee_challenges::tower::Prepared;
use baylee_gamehost::record::{Line, Source};
use central_api::tower;
use std::path::{Path, PathBuf};
#[path = "tower/stock.rs"]
mod stock;

fn evidence() -> Option<PathBuf> {
    std::env::var_os("BAYLEE_CENTRAL_TOWER_EVIDENCE").map(PathBuf::from)
}

fn ask(account: &Account, request: tower::Request) -> tower::Response {
    let api::Response::Tower { response } =
        account.challenges(api::Request::Tower { request }).unwrap()
    else {
        panic!("tower response")
    };
    *response
}

fn owned(account: &Account, request: tower::Request) -> tower::Run {
    let tower::Response::Run { run } = ask(account, request) else {
        panic!("owned expedition")
    };
    *run
}

fn read(account: &Account) -> tower::Response {
    ask(
        account,
        tower::Request::Read {
            season: None,
            board: tower::Board::Ascent,
            offset: 0,
        },
    )
}

pub(super) fn exercise(
    runtime: &Runtime,
    endpoint: &Endpoint,
    gateway: &common::Gateway,
    account: &Account,
    credential: &Token,
) {
    let Some(directory) = evidence() else {
        return;
    };
    let manifest =
        serde_json::from_slice(&std::fs::read(directory.join("manifest.json")).unwrap()).unwrap();
    let nodes = (0..tower::NODES)
        .map(|index| {
            serde_json::from_slice(
                &std::fs::read(directory.join(format!("node-{index:02}/publication.json")))
                    .unwrap(),
            )
            .unwrap()
        })
        .collect();
    let prepared = Prepared { manifest, nodes };
    let period = prepared.manifest.theme.period;
    prepared.validate(period).unwrap();
    let now = i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    )
    .unwrap();
    assert!(
        period.live_at(now),
        "the supplied evidence must belong to the current week"
    );
    publish(runtime, endpoint, credential, account, &prepared);
    let mut run = begin(account);
    let route = [
        tower::Track::Assault,
        tower::Track::Precision,
        tower::Track::Resolve,
        tower::Track::Assault,
    ];
    for floor in 0..5 {
        run = win(runtime, gateway, account, &directory, &run, floor == 4);
        assert_eq!(usize::from(run.progress.cleared), floor + 1);
        assert_eq!(run.progress.lives, 3);
        if let Some(&track) = route.get(floor) {
            let request = tower::Request::Choose {
                run: run.id,
                revision: run.revision,
                track,
                request_id: *Uuid::now_v7().as_bytes(),
            };
            run = owned(account, request.clone());
            let retry = owned(account, request);
            assert_eq!(retry.revision, run.revision);
            assert_eq!(retry.progress, run.progress);
        }
    }
    assert_eq!(run.progress.phase, tower::Phase::Conquered);
    assert_eq!(
        run.progress.milliseconds, None,
        "the resumed victory excludes a sprint"
    );
    let tower::Response::Read {
        standings,
        badges,
        active,
        ..
    } = read(account)
    else {
        panic!("tower board")
    };
    assert!(active.is_none());
    assert_eq!(standings.len(), 1);
    assert_eq!(
        (standings[0].floors, standings[0].losses, standings[0].rank),
        (5, 0, 1)
    );
    assert_eq!(
        badges.iter().map(|badge| badge.count).collect::<Vec<_>>(),
        [5, 1, 1]
    );
    let tower::Response::Read { standings, .. } = ask(
        account,
        tower::Request::Read {
            season: None,
            board: tower::Board::Sprint,
            offset: 0,
        },
    ) else {
        panic!("sprint board")
    };
    assert!(standings.is_empty());
    println!(
        "Tower: interrupted staging recovered, 35 nodes activated once, five network victories, four choices, reconnect and durable rewards verified."
    );
}

fn publish(
    runtime: &Runtime,
    endpoint: &Endpoint,
    credential: &Token,
    account: &Account,
    prepared: &Prepared,
) {
    let http = CentralHttp::new(endpoint.clone());
    let mut fail_after_commit = true;
    for pass in 0..3 {
        if pass > 0 {
            stock::resume(runtime, endpoint, credential, prepared, pass == 1);
            continue;
        }
        let result = baylee_challenges::tower::reconcile(
            prepared.manifest.theme.period.starts_at,
            1,
            |request| {
                let ambiguous = fail_after_commit
                    && matches!(
                        &request,
                        api::HostRequest::Tower {
                            request: tower::HostRequest::Stage { node: 7, .. }
                        }
                    );
                let wire::Response::ChallengeHost { response } = http.exchange(
                    &wire::Request::ChallengeHost { request },
                    Some(Bearer::Capability(credential)),
                )?
                else {
                    return Err(central_api::Error::Unavailable);
                };
                if ambiguous {
                    fail_after_commit = false;
                    return Err(central_api::Error::Unavailable);
                }
                Ok(response)
            },
            |period| {
                assert_eq!(period, prepared.manifest.theme.period);
                Ok(prepared.clone())
            },
        );
        if pass == 0 {
            assert!(result.is_err());
            let tower::Response::Read { season, .. } = read(account) else {
                panic!("partial stock")
            };
            assert!(
                season.is_none(),
                "partially staged towers are never published"
            );
        } else {
            assert_eq!(result.unwrap().published, usize::from(pass == 1));
        }
    }
}

fn begin(account: &Account) -> tower::Run {
    let tower::Response::Read {
        season: Some(season),
        ..
    } = read(account)
    else {
        panic!("published expedition")
    };
    let request = tower::Request::Begin {
        season: season.id,
        request_id: *Uuid::now_v7().as_bytes(),
        listed: true,
    };
    let run = owned(account, request.clone());
    assert_eq!(owned(account, request).id, run.id);
    run
}

fn win(
    runtime: &Runtime,
    gateway: &common::Gateway,
    account: &Account,
    directory: &Path,
    run: &tower::Run,
    reconnect: bool,
) -> tower::Run {
    let node = run.progress.node().unwrap();
    let run = owned(
        account,
        tower::Request::Start {
            run: run.id,
            revision: run.revision,
            request_id: *Uuid::now_v7().as_bytes(),
        },
    );
    let attempt = run.attempt.as_ref().unwrap();
    let admission = account
        .admit(&format!("http://127.0.0.1:{}", gateway.port))
        .unwrap();
    let path = format!("/challenges/{}/play", Uuid::from_bytes(attempt.id));
    let grant = start(gateway, &admission.token, &path);
    let game = *Uuid::parse_str(grant["game_id"].as_str().unwrap())
        .unwrap()
        .as_bytes();
    assert_eq!(owned(account, tower::Request::Locate { game }).id, run.id);
    let record =
        std::fs::read_to_string(directory.join(format!("node-{node:02}/proof-0.jsonl"))).unwrap();
    let actions = record
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
        .collect::<Vec<_>>();
    runtime.block_on(play(
        gateway,
        &admission.token,
        &path,
        &grant,
        &actions,
        reconnect,
    ));
    assert_eq!(
        wait_finished(account, attempt.id).state,
        api::AttemptState::Won
    );
    let finished = owned(account, tower::Request::Locate { game });
    assert_eq!(finished.id, run.id);
    assert_eq!(finished.progress.cleared, run.progress.cleared + 1);
    finished
}

pub(in crate::e2e_central) fn prepare_demo(
    runtime: &Runtime,
    gateway: &common::Gateway,
    account: &Account,
) {
    let Some(directory) = evidence() else {
        return;
    };
    let run = win(
        runtime,
        gateway,
        account,
        &directory,
        &begin(account),
        false,
    );
    assert_eq!(run.progress.phase, tower::Phase::Choosing);
}
