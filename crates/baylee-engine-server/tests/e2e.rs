//! End-to-end smoke test: the real server binary over a real socket.
//! Spawns `baylee-engine-server` on an ephemeral port, connects a
//! protobuf websocket client, creates the acceptance duel, answers a
//! mulligan, and verifies the game advances (a new choice arrives).

#![allow(clippy::missing_docs_in_private_items)]

use baylee_core::ids::PlayerId;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_protocol::v1::{self, Envelope};
use futures_util::{SinkExt, StreamExt};
use prost::Message;

/// The server binary on an ephemeral port, reaped whatever happens.
///
/// Each of the three tests below used to spawn a process and end with
/// `let _ = server.kill();`. Three servers were found listening on this
/// machine for nearly four hours, and nothing had said so — no test failed,
/// no port was taken, no log carried a line. Somebody read `ps`.
///
/// **Where the leak actually is** was measured rather than assumed, and the
/// first guess was wrong. Inverting all three assertions leaves *nothing*
/// behind even without a guard, because the assertions sit after the
/// websocket work and this server exits once its last client disconnects —
/// so the obvious counter-test passes for a reason that has nothing to do
/// with the cleanup. The leak is a death while no client has connected yet:
/// a panic placed between the spawn and the connect loop leaves **one**
/// process without this guard and **nought** with it, which is the measured
/// pair. An aborted run — a gate stopped with three tests in their connect
/// loops — is that same moment three times over, and is where the three
/// came from.
///
/// So the rule is not "kill at the end" but "kill on the way out, whichever
/// way that is", and `Drop` is the only thing that spans both.
struct Server {
    /// The running binary.
    child: std::process::Child,
    /// Where to dial it.
    url: String,
}

impl Server {
    /// Asks the kernel for a free port, releases it, and starts the binary
    /// on it.
    ///
    /// Probe-then-release is a race, and it is the one these tests have
    /// always run: the server takes a `PORT` and cannot be handed a bound
    /// listener, and asking for a fixed port instead would stop three of
    /// these from running at once.
    fn spawn() -> Self {
        let probe = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = probe.local_addr().unwrap().port();
        drop(probe);
        let child = std::process::Command::new(env!("CARGO_BIN_EXE_baylee-engine-server"))
            .env("PORT", port.to_string())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("spawn server");
        Self {
            child,
            url: format!("ws://127.0.0.1:{port}"),
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        // Reaped and not merely signalled. A killed child nobody waits for
        // is a zombie held by the test binary until the whole run ends, and
        // two of these three tests already knew that -- they called `wait`
        // after `kill` and the third did not.
        let _ = self.child.wait();
    }
}

/// How long anything here waits for something that **must** arrive.
///
/// One constant rather than a number per loop, because every one of those
/// numbers was chosen against an idle machine and they were all wrong in the
/// same way. This file had its own copy of the five-second budget #78 raised
/// in the gateway suite, and it went down the same way: gateway's full gate
/// took `a_socket_can_take_an_ai_chair_and_hand_it_back` with it at the dial,
/// before a line of the code under test had run. Alone the same test passes
/// in 1.57 s.
///
/// Thirty seconds is not a margin for a hang. Every loop that uses it exits on
/// the frame it was waiting for, so a raised ceiling costs a passing run
/// nothing and only changes which failures are real; a genuine hang still
/// fails, thirty seconds later, with the same message.
const WAIT_BUDGET: std::time::Duration = std::time::Duration::from_secs(30);

/// How many [`WAIT_STEP`]s fit in [`WAIT_BUDGET`].
const WAIT_TRIES: u32 = 300;

/// One poll interval, so a dial loop states its budget instead of its
/// arithmetic.
const WAIT_STEP: std::time::Duration = std::time::Duration::from_millis(100);

/// How long a socket is given to stay **quiet**, which is the opposite
/// question and does not take [`WAIT_BUDGET`].
///
/// A loop proving that nothing more arrives pays its whole budget on every
/// passing run, so thirty seconds here would be thirty seconds added to the
/// suite for each one. The trade is real and is stated rather than hidden:
/// this budget's failure mode is a false **pass** — a frame that should not
/// exist arriving late is never seen — where every other budget in this file
/// fails red. It is only ever used after the thing the test is actually
/// waiting for has already arrived.
const QUIET: u64 = 500;

#[tokio::test]
#[allow(clippy::too_many_lines)] // e2e scenario script
async fn create_game_and_answer_first_choice() {
    let server = Server::spawn();

    // Wait for the port to accept.
    let url = server.url.clone();
    let mut ws = None;
    for _ in 0..WAIT_TRIES {
        match tokio_tungstenite::connect_async(&url).await {
            Ok((stream, _)) => {
                ws = Some(stream);
                break;
            }
            Err(_) => tokio::time::sleep(WAIT_STEP).await,
        }
    }
    let mut ws = ws.expect("server accepts the websocket");

    // CreateGame → GameCreated (+ first choice request).
    let create = Envelope {
        msg: Some(v1::envelope::Msg::CreateGame(v1::CreateGame {
            preset: None,
        })),
    };
    ws.send(tokio_tungstenite::tungstenite::Message::Binary(
        create.encode_to_vec().into(),
    ))
    .await
    .expect("send create");

    let mut saw_game_created = false;
    let mut saw_view = false;
    let mut first_pending: Option<Pending> = None;
    let mut game_id = String::new();
    for _ in 0..10 {
        let Some(frame) = ws.next().await else { break };
        let frame = frame.expect("frame");
        if !frame.is_binary() {
            continue;
        }
        let env = Envelope::decode(frame.into_data()).expect("decode");
        match env.msg {
            Some(v1::envelope::Msg::GameCreated(created)) => {
                saw_game_created = true;
                game_id = created.game_id;
            }
            Some(v1::envelope::Msg::StateDelta(delta)) => {
                // Hidden information: the view exists and carries only
                // the viewing seat's own hand contents.
                let view: serde_json::Value =
                    serde_json::from_slice(&delta.view_json).expect("view json");
                saw_view = true;
                assert!(view.get("seats").is_some(), "view has seat lines");
                assert!(view.get("hand").is_some(), "view has the own hand");
            }
            Some(v1::envelope::Msg::ChoiceRequest(req)) => {
                first_pending = serde_json::from_slice(&req.pending_json).ok();
                break;
            }
            _ => {}
        }
    }
    assert!(saw_game_created, "server acknowledged the game");
    assert!(saw_view, "server sent a hidden-info-filtered view");
    let pending = first_pending.expect("server requested a choice");

    // Answer it; the game must advance with another choice (or game over).
    let action = match pending {
        Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
        _ => PlayerAction::PassPriority,
    };
    let answer = Envelope {
        msg: Some(v1::envelope::Msg::PlayerAction(v1::PlayerActionMsg {
            game_id: String::new(),
            seat_token: String::new(),
            action_json: serde_json::to_vec(&action).unwrap(),
        })),
    };
    ws.send(tokio_tungstenite::tungstenite::Message::Binary(
        answer.encode_to_vec().into(),
    ))
    .await
    .expect("send action");

    let mut advanced = false;
    for _ in 0..10 {
        let Some(frame) = ws.next().await else { break };
        let frame = frame.expect("frame");
        if !frame.is_binary() {
            continue;
        }
        let env = Envelope::decode(frame.into_data()).expect("decode");
        if matches!(
            env.msg,
            Some(v1::envelope::Msg::ChoiceRequest(_) | v1::envelope::Msg::GameCreated(_))
        ) {
            advanced = true;
            break;
        }
    }
    assert!(advanced, "the game advanced after the first answer");

    // A second connection joins the same game by id (multi-game manager).
    let (mut ws2, _) = tokio_tungstenite::connect_async(&url)
        .await
        .expect("second client connects");
    let join = Envelope {
        msg: Some(v1::envelope::Msg::Join(v1::JoinGame {
            game_id: game_id.clone(),
            seat_token: String::new(),
        })),
    };
    ws2.send(tokio_tungstenite::tungstenite::Message::Binary(
        join.encode_to_vec().into(),
    ))
    .await
    .expect("send join");
    let mut joined = false;
    for _ in 0..10 {
        let Some(frame) = ws2.next().await else { break };
        let frame = frame.expect("frame");
        if !frame.is_binary() {
            continue;
        }
        let env = Envelope::decode(frame.into_data()).expect("decode");
        if matches!(
            env.msg,
            Some(v1::envelope::Msg::StateDelta(_) | v1::envelope::Msg::ChoiceRequest(_))
        ) {
            joined = true;
            break;
        }
    }
    assert!(joined, "a second client re-attached to the live game");
}

/// The AI chair is a chair: a socket can sit in it, is asked the questions
/// the house AI would have answered, and gives it back by hanging up.
///
/// This is the rules-side counterpart of the client's `dev-control` harness,
/// and it is written over two real sockets on purpose. A test that called
/// `Session::take_over` directly would pass while the harness still routed
/// every seat's envelopes to whichever socket happened to be holding the
/// lock — which is exactly what it did before this existed.
#[tokio::test]
#[allow(clippy::too_many_lines)] // e2e scenario script
async fn a_socket_can_take_an_ai_chair_and_hand_it_back() {
    let server = Server::spawn();
    let url = server.url.clone();
    let mut human = dial(&url).await;

    // Seat 0 creates the dev duel and is asked the first question.
    send(
        &mut human,
        &Envelope {
            msg: Some(v1::envelope::Msg::CreateGame(v1::CreateGame {
                preset: None,
            })),
        },
    )
    .await;
    let mut game_id = String::new();
    let mut asked_seat_0 = None;
    for _ in 0..10 {
        let Some(env) = next(&mut human).await else {
            break;
        };
        match env.msg {
            Some(v1::envelope::Msg::GameCreated(created)) => game_id = created.game_id,
            Some(v1::envelope::Msg::ChoiceRequest(req)) => {
                asked_seat_0 = serde_json::from_slice::<Pending>(&req.pending_json).ok();
                break;
            }
            _ => {}
        }
    }
    assert!(!game_id.is_empty(), "the game was created");
    let asked_seat_0 = asked_seat_0.expect("seat 0 was asked something");

    // A second socket takes seat 1 — the house AI's chair.
    let mut driver = dial(&url).await;
    send(
        &mut driver,
        &Envelope {
            msg: Some(v1::envelope::Msg::Join(v1::JoinGame {
                game_id: game_id.clone(),
                seat_token: "1".to_string(),
            })),
        },
    )
    .await;
    let mut driver_saw_view = false;
    let mut driver_asked = false;
    for _ in 0..10 {
        // The full budget until the view this asserts on has arrived, and
        // `QUIET` only afterwards: the 500 ms this used to spend waiting for
        // it was a statement about the machine.
        let budget = if driver_saw_view {
            QUIET
        } else {
            WAIT_BUDGET.as_millis() as u64
        };
        let Some(env) = next_within(&mut driver, budget).await else {
            break;
        };
        match env.msg {
            Some(v1::envelope::Msg::StateDelta(delta)) => {
                assert_eq!(
                    view_seat(&delta),
                    Some(PlayerId::new(1)),
                    "the driven socket is sent its own seat's view"
                );
                driver_saw_view = true;
            }
            Some(v1::envelope::Msg::ChoiceRequest(_)) => {
                driver_asked = true;
                break;
            }
            Some(v1::envelope::Msg::Error(err)) => panic!("seat 1 was refused: {}", err.message),
            _ => {}
        }
    }
    assert!(driver_saw_view, "seat 1 is sent its own view");

    // While this socket holds the chair, a second one may not have it. Two
    // programs on one chair would race each other for every question.
    let mut queue_jumper = dial(&url).await;
    send(
        &mut queue_jumper,
        &Envelope {
            msg: Some(v1::envelope::Msg::Join(v1::JoinGame {
                game_id: game_id.clone(),
                seat_token: "1".to_string(),
            })),
        },
    )
    .await;
    let mut second_refused = false;
    for _ in 0..10 {
        // Same split: the refusal is what this asserts on, so it is waited
        // for with the full budget and only the tail is quiet.
        let budget = if second_refused {
            QUIET
        } else {
            WAIT_BUDGET.as_millis() as u64
        };
        let Some(env) = next_within(&mut queue_jumper, budget).await else {
            break;
        };
        match env.msg {
            Some(v1::envelope::Msg::Error(_)) => {
                second_refused = true;
                break;
            }
            Some(v1::envelope::Msg::StateDelta(_) | v1::envelope::Msg::ChoiceRequest(_)) => {
                panic!("a second socket was served the chair someone is already driving")
            }
            _ => {}
        }
    }
    assert!(second_refused, "a chair already being driven is refused");
    drop(queue_jumper);

    // Seat 0 answers until a question belongs to seat 1, and that one must
    // come out of the socket rather than into `HeuristicAgent::act`. The
    // house kept seat 1's hand while seat 0 was still deciding its own (every
    // seat is asked its mulligan at once), so that is turn 1's first priority
    // pass rather than a mulligan.
    let mut seat_0_asked = Some(asked_seat_0);
    for _ in 0..10 {
        if driver_asked {
            break;
        }
        let Some(pending) = seat_0_asked.take() else {
            break;
        };
        answer(&mut human, &pending).await;
        let asked = tokio::time::timeout(WAIT_BUDGET, async {
            loop {
                tokio::select! {
                    env = next(&mut driver) => match env?.msg {
                        Some(v1::envelope::Msg::StateDelta(delta)) => assert_eq!(
                            view_seat(&delta),
                            Some(PlayerId::new(1)),
                            "the driven socket is sent its own seat's view"
                        ),
                        Some(v1::envelope::Msg::ChoiceRequest(_)) => return Some(Asked::Driver),
                        _ => {}
                    },
                    env = next(&mut human) => match env?.msg {
                        Some(v1::envelope::Msg::StateDelta(delta)) => assert_eq!(
                            view_seat(&delta),
                            Some(PlayerId::new(0)),
                            "seat 0 was sent another seat's view"
                        ),
                        Some(v1::envelope::Msg::ChoiceRequest(req)) => {
                            let pending = serde_json::from_slice(&req.pending_json)
                                .expect("seat 0's question decodes");
                            return Some(Asked::Human(pending));
                        }
                        _ => {}
                    },
                }
            }
        })
        .await
        .ok()
        .flatten();
        match asked {
            Some(Asked::Driver) => driver_asked = true,
            Some(Asked::Human(pending)) => seat_0_asked = Some(pending),
            None => break,
        }
    }
    assert!(
        driver_asked,
        "the taken-over seat is asked instead of answering itself"
    );

    // Nothing is left in flight for seat 0 before the chair goes back — and
    // nothing that arrived belonged to seat 1. This is the routing
    // assertion, and it is the reason the fan carries a seat number at all:
    // a fan that copied every frame to every socket would hand seat 0 the
    // driven seat's view of its own hand, and every take-over test above
    // would still pass.
    while let Some(env) = next_within(&mut human, QUIET).await {
        if let Some(v1::envelope::Msg::StateDelta(delta)) = env.msg {
            assert_eq!(
                view_seat(&delta),
                Some(PlayerId::new(0)),
                "seat 0 was sent another seat's view"
            );
        }
    }

    // Hanging up hands the chair back: the house AI answers the question this
    // socket was sitting on and plays seat 1 forward until seat 0 is needed.
    drop(driver);
    let mut human_asked_again = false;
    for _ in 0..20 {
        let Some(env) = next(&mut human).await else {
            break;
        };
        if matches!(env.msg, Some(v1::envelope::Msg::ChoiceRequest(_))) {
            human_asked_again = true;
            break;
        }
    }
    assert!(
        human_asked_again,
        "releasing the chair let the house AI play seat 1 on"
    );
}

/// Naming a seat that is not at the table is refused rather than served.
#[tokio::test]
async fn a_socket_cannot_sit_at_a_seat_that_is_not_there() {
    let server = Server::spawn();
    let url = server.url.clone();
    let mut human = dial(&url).await;
    send(
        &mut human,
        &Envelope {
            msg: Some(v1::envelope::Msg::CreateGame(v1::CreateGame {
                preset: None,
            })),
        },
    )
    .await;
    let mut game_id = String::new();
    for _ in 0..10 {
        let Some(env) = next(&mut human).await else {
            break;
        };
        if let Some(v1::envelope::Msg::GameCreated(created)) = env.msg {
            game_id = created.game_id;
            break;
        }
    }
    assert!(!game_id.is_empty(), "the game was created");

    let mut stranger = dial(&url).await;
    send(
        &mut stranger,
        &Envelope {
            msg: Some(v1::envelope::Msg::Join(v1::JoinGame {
                game_id,
                seat_token: "7".to_string(),
            })),
        },
    )
    .await;
    let mut refused = false;
    for _ in 0..10 {
        let Some(env) = next(&mut stranger).await else {
            break;
        };
        match env.msg {
            Some(v1::envelope::Msg::Error(_)) => {
                refused = true;
                break;
            }
            Some(v1::envelope::Msg::StateDelta(_) | v1::envelope::Msg::ChoiceRequest(_)) => {
                panic!("seat 7 was served a view at a two-seat table")
            }
            _ => {}
        }
    }
    assert!(refused, "a seat that is not at the table is refused");
}

type Client =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

async fn dial(url: &str) -> Client {
    for _ in 0..WAIT_TRIES {
        if let Ok((stream, _)) = tokio_tungstenite::connect_async(url).await {
            return stream;
        }
        tokio::time::sleep(WAIT_STEP).await;
    }
    panic!("server accepts the websocket");
}

async fn send(ws: &mut Client, envelope: &Envelope) {
    ws.send(tokio_tungstenite::tungstenite::Message::Binary(
        envelope.encode_to_vec().into(),
    ))
    .await
    .expect("send");
}

/// The next envelope, or `None` once the socket has gone quiet.
///
/// Quiet is an answer here, not a hang: several of the assertions above are
/// about a frame *arriving*, so the wait has to end on its own.
async fn next(ws: &mut Client) -> Option<Envelope> {
    next_within(ws, WAIT_BUDGET.as_millis() as u64).await
}

/// The same, with the wait spelled out — short when the point is that the
/// socket has nothing left to say.
async fn next_within(ws: &mut Client, millis: u64) -> Option<Envelope> {
    loop {
        let frame = tokio::time::timeout(std::time::Duration::from_millis(millis), ws.next()).await;
        match frame {
            Ok(Some(Ok(frame))) if frame.is_binary() => {
                return Envelope::decode(frame.into_data()).ok();
            }
            Ok(Some(Ok(_))) => {}
            Ok(Some(Err(_)) | None) | Err(_) => return None,
        }
    }
}

/// The seat a view was built for.
///
/// `PlayerView::seat` is what makes the routing checkable from outside the
/// process: a view says whose it is, so a socket receiving one that is not
/// its own is a leak and not merely noise.
fn view_seat(delta: &v1::StateDelta) -> Option<PlayerId> {
    let view: serde_json::Value = serde_json::from_slice(&delta.view_json).ok()?;
    serde_json::from_value(view.get("seat")?.clone()).ok()
}

/// Which of two sockets was asked the next question.
enum Asked {
    /// The socket driving an AI chair.
    Driver,
    /// Seat 0, and what it was asked.
    Human(Pending),
}

/// Answers a pending with the most trivial legal reply it has.
async fn answer(ws: &mut Client, pending: &Pending) {
    let action = match pending {
        Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
        _ => PlayerAction::PassPriority,
    };
    send(
        ws,
        &Envelope {
            msg: Some(v1::envelope::Msg::PlayerAction(v1::PlayerActionMsg {
                game_id: String::new(),
                seat_token: String::new(),
                action_json: serde_json::to_vec(&action).unwrap(),
            })),
        },
    )
    .await;
}
