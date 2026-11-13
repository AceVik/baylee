//! Certified generation → real Central HTTP → authenticated gateway → real engine
//! over its socket → durable result queue → rankings and permanent badges.
use super::{Endpoint, common};
use baylee_account::{Account, Bearer, CentralHttp, Token, wire};
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_protocol::v1::{self, Envelope};
use central_api::challenges as api;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::runtime::Runtime;
use uuid::Uuid;
#[path = "challenges/checks.rs"]
mod checks;

pub(super) fn run(
    runtime: &Runtime,
    endpoint: &Endpoint,
    gateway: &common::Gateway,
    account: &Account,
    credential: &Token,
) {
    if std::env::var("BAYLEE_CENTRAL_CHALLENGE_UI").as_deref() == Ok("1") {
        prepare_blind(runtime, endpoint, credential);
        return;
    }
    let daily = baylee_train::challenges::generate(
        api::Kind::Daily,
        41,
        baylee_train::challenges::Limits::default(),
    )
    .unwrap();
    let weekly = baylee_train::challenges::generate(
        api::Kind::Weekly,
        63,
        baylee_train::challenges::Limits::default(),
    )
    .unwrap();
    let http = CentralHttp::new(endpoint.clone());
    let now = i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    )
    .unwrap();
    let publish = || {
        baylee_challenges::stock::reconcile(
            now,
            1,
            |request| match http.exchange(
                &wire::Request::ChallengeHost { request },
                Some(Bearer::Capability(credential)),
            )? {
                wire::Response::ChallengeHost { response } => Ok(response),
                _ => Err(central_api::Error::Unavailable),
            },
            |period| {
                let candidate = if period.kind == api::Kind::Daily {
                    &daily
                } else {
                    &weekly
                };
                candidate
                    .publication(period)
                    .map_err(|_| baylee_challenges::stock::Error::Uncertified)
            },
        )
        .unwrap()
    };
    assert_eq!(
        publish().published,
        baylee_challenges::stock::periods(now, 1).unwrap().len()
    );
    assert_eq!(
        publish().published,
        0,
        "a repeated automatic pass publishes nothing twice"
    );
    let api::Response::Catalogue { challenges, .. } = account
        .challenges(api::Request::Catalogue { before: None })
        .unwrap()
    else {
        panic!("catalogue");
    };
    let mut agent = None;
    let done = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        for candidate in [&daily, &weekly] {
            let task = challenges
                .iter()
                .find(|task| task.description.period.kind == candidate.kind)
                .unwrap();
            checks::exercise(runtime, gateway, account, task, candidate, &mut agent);
        }
        let api::Response::Progress { badges, active } =
            account.challenges(api::Request::Progress).unwrap()
        else {
            panic!("badges");
        };
        assert!(active.is_empty());
        assert!(badges.iter().filter(|badge| badge.count > 0).count() >= 3);
    }));
    if let Some(agent) = agent {
        agent.abort();
        let _ = runtime.block_on(agent);
    }
    if let Err(error) = done {
        std::panic::resume_unwind(error);
    }
}

fn prepare_blind(runtime: &Runtime, endpoint: &Endpoint, credential: &Token) {
    let binary = std::env::var_os("BAYLEE_CHALLENGE_BINARY").expect("owned publisher binary");
    let directory =
        std::env::var_os("BAYLEE_CHALLENGE_STATE_DIR").expect("owned evidence directory");
    for pass in 0..2 {
        let output = runtime.block_on(async {
            let mut command = tokio::process::Command::new(&binary);
            command
                .args([
                    "stock",
                    "--central-dev",
                    endpoint.origin(),
                    "--days",
                    "1",
                    "--source-dir",
                ])
                .arg(baylee_train::working::repo_root())
                .arg("--state-dir")
                .arg(&directory)
                .env("BAYLEE_CHALLENGE_BRIDGE_TOKEN", &*credential.hex())
                .kill_on_drop(true);
            tokio::time::timeout(Duration::from_secs(600), command.output())
                .await
                .expect("bounded automatic challenge generation")
                .expect("publisher process")
        });
        assert!(
            output.status.success(),
            "automatic publisher failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        if pass == 1 {
            assert!(String::from_utf8_lossy(&output.stdout).contains(", 0 published"));
        }
    }
    // Deliberately never open the private proof records: this fixture is for blind play.
}

fn start(gateway: &common::Gateway, token: &str, path: &str) -> serde_json::Value {
    let (status, body) = common::http(gateway.port, "POST", path, Some(token), "{}");
    assert_eq!(status, 200, "challenge start: {body}");
    serde_json::from_str(&body).unwrap()
}

fn wait_finished(account: &Account, id: [u8; 16]) -> api::Attempt {
    for _ in 0..100 {
        let api::Response::Attempt { attempt } = account
            .challenges(api::Request::Attempt { attempt: id })
            .unwrap()
        else {
            panic!("attempt");
        };
        if !attempt.state.active() {
            return attempt;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    panic!("the durable result was not delivered");
}

async fn play(
    gateway: &common::Gateway,
    token: &str,
    path: &str,
    grant: &serde_json::Value,
    actions: &[PlayerAction],
    reconnect: bool,
) {
    let game = grant["game_id"].as_str().unwrap();
    let mut socket =
        common::dial_seat(gateway.port, game, grant["seat_token"].as_str().unwrap()).await;
    let mut sent = 0;
    let mut resumed = false;
    let mut sequence = None;
    tokio::time::timeout(Duration::from_secs(90), async {
        while let Some(message) = common::next_msg(&mut socket).await {
            match message {
                v1::envelope::Msg::GameStatic(_) => {
                    common::send(
                        &mut socket,
                        &Envelope {
                            msg: Some(v1::envelope::Msg::SeatReady(v1::SeatReady {})),
                        },
                    )
                    .await;
                }
                v1::envelope::Msg::ChoiceRequest(question) => {
                    let pending: Pending = baylee_core::wire::decode(&question.pending).unwrap();
                    if matches!(pending, Pending::GameOver(_)) {
                        break;
                    }
                    if reconnect && sent == 1 && !resumed {
                        socket.close(None).await.unwrap();
                        let again = start(gateway, token, path);
                        assert_eq!(again["game_id"], grant["game_id"]);
                        socket = common::dial_seat(
                            gateway.port,
                            game,
                            again["seat_token"].as_str().unwrap(),
                        )
                        .await;
                        resumed = true;
                        sequence = None;
                        continue;
                    }
                    if sequence == Some(question.seq) {
                        continue;
                    }
                    sequence = Some(question.seq);
                    let action = actions
                        .get(sent)
                        .expect("a verified answer for this question");
                    common::send(
                        &mut socket,
                        &Envelope {
                            msg: Some(v1::envelope::Msg::PlayerAction(v1::PlayerActionMsg {
                                game_id: String::new(),
                                seat_token: String::new(),
                                action: baylee_core::wire::encode(action),
                            })),
                        },
                    )
                    .await;
                    sent += 1;
                }
                v1::envelope::Msg::Error(error) => {
                    panic!("verified move refused: {}", error.message)
                }
                v1::envelope::Msg::GameRecordChunk(_) => panic!("private record leaked to player"),
                _ => {}
            }
        }
    })
    .await
    .expect("the real network game completes");
    let _ = socket.close(None).await;
    assert_eq!(sent, actions.len());
    assert_eq!(resumed, reconnect);
}
