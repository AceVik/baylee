//! The networked host, against a real websocket server.
//!
//! Everything else about a duel is testable as arithmetic; the transport is
//! not. So this spins up a socket, puts a real [`Session`] behind it, and
//! plays through [`NetworkHost`] exactly as the gateway would be played
//! through — including the reconnect, which is the part that only ever runs
//! when something has already gone wrong.

#![allow(clippy::missing_docs_in_private_items)]

use baylee_client::host::{DuelHost, HostMessage, LinkState};
use baylee_client::{NetworkHost, SeatTicket};
use baylee_core::ids::{CardIndex, PlayerId, PrintRef};
use baylee_core::preset::{
    AIProfile, DeckEntry, Finish, FormatId, GamePreset, HouseRules, PrintInfo, SeatCapabilities,
    SeatController, SeatSpec,
};
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_gamehost::Session;
use baylee_protocol::v1::{self, Envelope};
use futures_util::{SinkExt, StreamExt};
use prost::Message as _;

/// The seat the test plays.
const SEAT: PlayerId = PlayerId::new(0);
/// The game id the table hands out.
const GAME: &str = "test-table";
/// The seat token the lobby handed this seat: what buys its socket tickets.
const SEAT_TOKEN: &str = "0123456789abcdef";
/// Seat names, so the roster is checkable.
fn names() -> Vec<String> {
    vec!["You".to_string(), "House AI".to_string()]
}

fn island() -> CardIndex {
    baylee_cards::by_oracle_id("b2c6aa39-2d2a-459c-a555-fb48ba993373")
        .expect("Island is in the registry")
        .index
}

/// A duel of two Island decks: human on seat 0, house AI on seat 1.
fn duel_preset() -> GamePreset {
    let deck: Vec<DeckEntry> = (0..60)
        .map(|_| DeckEntry {
            card: island(),
            print: PrintRef::new(0),
        })
        .collect();
    let seat = |ai: bool| SeatSpec {
        controller: if ai {
            SeatController::Ai(AIProfile::default())
        } else {
            SeatController::Open
        },
        capabilities: SeatCapabilities::default(),
        deck: deck.clone(),
        sideboard: vec![],
        commanders: vec![],
        starting_life: None,
        starting_hand: None,
        starting_battlefield: vec![],
        emblems: vec![],
        team: None,
    };
    GamePreset {
        format: FormatId::Freeform,
        seed: 7,
        house_rules: HouseRules::default(),
        modifiers: vec![],
        prints: vec![PrintInfo {
            scryfall_id: uuid::Uuid::nil(),
            lang: "EN".into(),
            finish: Finish::Normal,
        }],
        seats: vec![seat(false), seat(true)],
    }
}

/// How the fake gateway in front of the table behaves at its door (#294).
#[derive(Clone, Copy, Default)]
struct Door {
    /// How many upgrades to refuse as a stale ticket before letting one in,
    /// however good the ticket.
    refuse_upgrades: usize,
    /// Answer every ticket request with this status instead of a ticket.
    ticket_status: Option<u16>,
}

/// What the fake gateway saw: every ticket request's `Authorization`
/// header and every upgrade's path and query.
#[derive(Default)]
struct Seen {
    ticket_requests: Vec<String>,
    upgrades: Vec<String>,
    issued: Vec<String>,
}

type Witness = std::sync::Arc<std::sync::Mutex<Seen>>;

/// Starts a table on a free port and returns it.
///
/// Connections are served one at a time on purpose: the reconnect test needs
/// the *same* session to still be there when the second socket arrives, which
/// is exactly what a gateway guarantees and a fresh game would not.
fn spawn_table() -> u16 {
    spawn_table_behind(Door::default()).0
}

/// The same, behind a door that sells single-use tickets as the gateway does
/// (#294): `POST /ws-ticket` with the seat token as a bearer, then the
/// upgrade with `?ticket=`, spent on use.
fn spawn_table_behind(door: Door) -> (u16, Witness) {
    spawn_table_with_preset(door, duel_preset())
}

fn spawn_table_with_preset(door: Door, preset: GamePreset) -> (u16, Witness) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    listener.set_nonblocking(true).expect("nonblocking");
    let seen = Witness::default();
    let witness = seen.clone();
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        runtime.block_on(async move {
            let listener = tokio::net::TcpListener::from_std(listener).expect("listener");
            let mut session = Session::new(&preset).expect("session");
            session.describe(GAME.to_string(), names());
            let mut refusals = door.refuse_upgrades;
            while let Ok((stream, _)) = listener.accept().await {
                let mut head = [0u8; 4];
                if stream.peek(&mut head).await.is_ok() && &head == b"POST" {
                    sell_ticket(stream, door, &seen).await;
                    continue;
                }
                let Some(ws) = admit(stream, &seen, &mut refusals).await else {
                    continue;
                };
                serve(ws, &mut session).await;
            }
        });
    });
    (port, witness)
}

/// Answers one `POST /ws-ticket`: a fresh ticket for the seat token this
/// test hands out, a `401` for any other bearer.
async fn sell_ticket(mut stream: tokio::net::TcpStream, door: Door, seen: &Witness) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut raw = Vec::new();
    let mut buf = [0u8; 1024];
    let (head, start, length) = loop {
        let Ok(n) = stream.read(&mut buf).await else {
            return;
        };
        if n == 0 {
            return;
        }
        raw.extend_from_slice(&buf[..n]);
        let text = String::from_utf8_lossy(&raw).into_owned();
        if let Some(end) = text.find("\r\n\r\n") {
            let length = text[..end]
                .lines()
                .find_map(|l| {
                    l.to_ascii_lowercase()
                        .strip_prefix("content-length:")
                        .map(|v| v.trim().parse::<usize>().unwrap_or(0))
                })
                .unwrap_or(0);
            break (text[..end].to_string(), end + 4, length);
        }
    };
    while raw.len() < start + length {
        let Ok(n) = stream.read(&mut buf).await else {
            return;
        };
        if n == 0 {
            break;
        }
        raw.extend_from_slice(&buf[..n]);
    }
    let body = String::from_utf8_lossy(&raw[start.min(raw.len())..(start + length).min(raw.len())])
        .into_owned();
    let bearer = head
        .lines()
        .find_map(|l| {
            let (name, value) = l.split_once(':')?;
            name.eq_ignore_ascii_case("authorization")
                .then(|| value.trim().to_string())
        })
        .unwrap_or_default();
    let path_ok = head.starts_with(&format!("POST {} ", baylee_protocol::WS_TICKET_PATH));
    let (status, answer) = {
        let mut seen = seen.lock().expect("witness");
        seen.ticket_requests.push(bearer.clone());
        match door.ticket_status {
            Some(status) => (status, r#"{"error":"no"}"#.to_string()),
            None if path_ok
                && bearer == format!("Bearer {SEAT_TOKEN}")
                && body.contains(r#""socket":"seat""#)
                && body.contains(GAME) =>
            {
                let ticket = format!("tkt{}", seen.issued.len());
                seen.issued.push(ticket.clone());
                (200, format!(r#"{{"ticket":"{ticket}","expires_in":45}}"#))
            }
            None => (401, r#"{"error":"invalid seat token"}"#.to_string()),
        }
    };
    let reply = format!(
        "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{answer}",
        answer.len()
    );
    let _ = stream.write_all(reply.as_bytes()).await;
    let _ = stream.shutdown().await;
}

/// The upgrade: let in only a ticket this door sold and nobody has spent,
/// unless it was told to refuse the next few whatever they carry.
#[allow(clippy::result_large_err)] // tungstenite's handshake callback, as it is declared
async fn admit(
    stream: tokio::net::TcpStream,
    seen: &Witness,
    refusals: &mut usize,
) -> Option<tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>> {
    use tokio_tungstenite::tungstenite::http;
    let refuse = if *refusals > 0 {
        *refusals -= 1;
        true
    } else {
        false
    };
    let seen = seen.clone();
    tokio_tungstenite::accept_hdr_async(
        stream,
        move |request: &tokio_tungstenite::tungstenite::handshake::server::Request, response| {
            let asked = request.uri().to_string();
            let ticket = request
                .uri()
                .query()
                .unwrap_or_default()
                .split('&')
                .find_map(|pair| pair.strip_prefix("ticket="))
                .unwrap_or_default()
                .to_string();
            let mut seen = seen.lock().expect("witness");
            seen.upgrades.push(asked);
            // Spent on presentation, as the gateway spends it.
            let known = seen.issued.iter().position(|t| *t == ticket);
            let sold = known.is_some_and(|i| {
                seen.issued[i] = String::new();
                true
            });
            if sold && !refuse {
                Ok(response)
            } else {
                Err(http::Response::builder()
                    .status(401)
                    .body(Some(format!(
                        r#"{{"error":"{}"}}"#,
                        baylee_protocol::TICKET_REFUSED
                    )))
                    .expect("a response"))
            }
        },
    )
    .await
    .ok()
}

/// One connection: the opening payload, then answers until the socket closes.
async fn serve(
    mut ws: tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
    session: &mut Session,
) {
    let mut opening = vec![session.game_static_envelope(SEAT)];
    opening.extend(mine(session.pump()));
    for envelope in opening {
        if send(&mut ws, envelope).await.is_err() {
            return;
        }
    }
    // Whether this socket said a person answers it, as the engine would
    // write into the record, before it said it is ready.
    let mut declared_human = false;
    while let Some(Ok(frame)) = ws.next().await {
        if !frame.is_binary() {
            continue;
        }
        let Ok(envelope) = Envelope::decode(frame.into_data()) else {
            continue;
        };
        let replies = match envelope.msg {
            Some(v1::envelope::Msg::SeatMind(mind)) => {
                declared_human = mind.kind == v1::seat_mind::Kind::Human as i32
                    && baylee_protocol::mind::fault(&mind).is_none();
                vec![]
            }
            Some(v1::envelope::Msg::PlayerAction(msg)) => {
                let action: PlayerAction =
                    serde_json::from_slice(&msg.action_json).expect("an action decodes");
                match session.act(SEAT, action) {
                    Ok(routed) => mine(routed),
                    Err(reason) => vec![Envelope {
                        msg: Some(v1::envelope::Msg::Error(v1::Error {
                            code: 1,
                            message: reason,
                        })),
                    }],
                }
            }
            Some(v1::envelope::Msg::Resume(msg)) => session.resume(SEAT, msg.last_seq),
            Some(v1::envelope::Msg::ClockProbe(probe)) => vec![Envelope {
                msg: Some(v1::envelope::Msg::ClockProbe(v1::ClockProbe {
                    client_time_ms: probe.client_time_ms,
                    server_time_ms: 100_000,
                })),
            }],
            // A table of one: the seat that is ready is the last one. One
            // that did not first say a person answers it is not let in.
            Some(v1::envelope::Msg::SeatReady(_)) if declared_human => vec![Envelope {
                msg: Some(v1::envelope::Msg::Curtain(v1::Curtain {})),
            }],
            Some(v1::envelope::Msg::SeatReady(_)) => vec![Envelope {
                msg: Some(v1::envelope::Msg::Error(v1::Error {
                    code: 1,
                    message: "ready before saying what answers the seat".into(),
                })),
            }],
            _ => vec![],
        };
        for envelope in replies {
            if send(&mut ws, envelope).await.is_err() {
                return;
            }
        }
    }
}

/// The envelopes addressed to the seat this test plays.
fn mine(routed: Vec<(PlayerId, Envelope)>) -> Vec<Envelope> {
    routed
        .into_iter()
        .filter(|(player, _)| *player == SEAT)
        .map(|(_, envelope)| envelope)
        .collect()
}

async fn send(
    ws: &mut tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
    envelope: Envelope,
) -> Result<(), ()> {
    ws.send(tokio_tungstenite::tungstenite::Message::Binary(
        envelope.encode_to_vec().into(),
    ))
    .await
    .map_err(|_| ())
}

fn ticket(port: u16) -> SeatTicket {
    SeatTicket {
        gateway: format!("http://127.0.0.1:{port}"),
        game_id: GAME.to_string(),
        seat: SEAT,
        seat_token: SEAT_TOKEN.to_string(),
    }
}

/// How long anything here waits for something that **must** arrive.
///
/// The third copy of the budget #78 raised in the gateway suite and #97 in
/// `baylee-engine-server`'s, written — like both of those — against a machine
/// with nothing else on it. Five worktrees share this one, and a socket
/// opening beside another tree's compile is not a ten-second event.
///
/// Every loop below exits on the message it was waiting for, so the raised
/// ceiling costs a passing run nothing and only changes which failures are
/// real: a genuine hang still fails, later, with the same words.
const WAIT_BUDGET: std::time::Duration = std::time::Duration::from_secs(30);

/// How many [`WAIT_STEP`]s fit in [`WAIT_BUDGET`].
const WAIT_TRIES: u32 = 3000;

/// One poll interval, so a loop states its budget instead of its arithmetic.
const WAIT_STEP: std::time::Duration = std::time::Duration::from_millis(10);

/// Polls the host until the messages so far satisfy `done`.
///
/// A frame loop with a deadline, which is what the client itself does: the
/// host may never block, so waiting is the caller's job.
fn poll_until<F>(host: &mut NetworkHost, what: &str, mut done: F) -> Vec<HostMessage>
where
    F: FnMut(&[HostMessage]) -> bool,
{
    let mut all = Vec::new();
    for _ in 0..WAIT_TRIES {
        all.extend(host.poll());
        if done(&all) {
            return all;
        }
        std::thread::sleep(WAIT_STEP);
    }
    panic!("waited {WAIT_BUDGET:?} for {what}; got {all:#?}");
}

fn statics(messages: &[HostMessage]) -> Vec<&baylee_view::GameStatic> {
    messages
        .iter()
        .filter_map(|m| match m {
            HostMessage::Static(s) => Some(&**s),
            _ => None,
        })
        .collect()
}

fn views(messages: &[HostMessage]) -> Vec<&baylee_view::PlayerView> {
    messages
        .iter()
        .filter_map(|m| match m {
            HostMessage::View(v, _) => Some(&**v),
            _ => None,
        })
        .collect()
}

fn choices(messages: &[HostMessage]) -> Vec<&Pending> {
    messages
        .iter()
        .filter_map(|m| match m {
            HostMessage::Choice(p) => Some(&**p),
            _ => None,
        })
        .collect()
}

/// A table that refuses this client's protocol, as the gateway does
/// (#271): it hands back the query each socket was opened with, sends the
/// one frame the gateway would, and closes.
#[allow(clippy::result_large_err)] // tungstenite's handshake callback, as it is declared
fn spawn_refusing_table(table: u32) -> (u16, std::sync::mpsc::Receiver<String>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    listener.set_nonblocking(true).expect("nonblocking");
    let (asked, heard) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        runtime.block_on(async move {
            let listener = tokio::net::TcpListener::from_std(listener).expect("listener");
            let tickets = Witness::default();
            while let Ok((stream, _)) = listener.accept().await {
                let mut head = [0u8; 4];
                if stream.peek(&mut head).await.is_ok() && &head == b"POST" {
                    sell_ticket(stream, Door::default(), &tickets).await;
                    continue;
                }
                let asked = asked.clone();
                let Ok(mut ws) = tokio_tungstenite::accept_hdr_async(
                    stream,
                    move |request: &tokio_tungstenite::tungstenite::handshake::server::Request,
                          response| {
                        let _ = asked.send(request.uri().query().unwrap_or_default().to_string());
                        Ok(response)
                    },
                )
                .await
                else {
                    continue;
                };
                let refusal = Envelope {
                    msg: Some(v1::envelope::Msg::HelloAck(v1::HelloAck {
                        protocol_version: table,
                        compatible: false,
                        message: "This client speaks another protocol.".to_string(),
                    })),
                };
                if send(&mut ws, refusal).await.is_ok() {
                    let _ = ws.close(None).await;
                }
            }
        });
    });
    (port, heard)
}

/// A seat socket says which protocol it speaks, and a table that refuses
/// it is final (#271): the host shows the refusal as its link, and dials
/// that table no more.
#[test]
fn a_table_that_refuses_the_protocol_is_not_dialled_again() {
    let table = baylee_protocol::PROTOCOL_VERSION + 1;
    let (port, asked) = spawn_refusing_table(table);
    let mut host = NetworkHost::connect(ticket(port)).expect("connect");
    // Polled while waiting: the socket is opened by the frame that picks up
    // its ticket (#294), not by `connect`.
    let mut query = None;
    for _ in 0..WAIT_TRIES {
        host.poll();
        if let Ok(asked) = asked.try_recv() {
            query = Some(asked);
            break;
        }
        std::thread::sleep(WAIT_STEP);
    }
    let query = query.expect("the table was dialled");
    assert!(
        query.ends_with(&format!("&protocol={}", baylee_protocol::PROTOCOL_VERSION)),
        "the socket did not say its protocol: {query}"
    );
    for _ in 0..WAIT_TRIES {
        host.poll();
        if host.link() == (LinkState::Refused { table }) {
            break;
        }
        std::thread::sleep(WAIT_STEP);
    }
    assert_eq!(
        host.link(),
        LinkState::Refused { table },
        "after {WAIT_BUDGET:?}"
    );
    assert!(host.reconnect().is_err(), "a refused table is not dialled");
    assert_eq!(
        host.link(),
        LinkState::Refused { table },
        "and stays refused"
    );
    assert!(
        asked
            .recv_timeout(std::time::Duration::from_millis(200))
            .is_err(),
        "nothing dialled it again"
    );
}

/// `ready` reaches the table as a `SeatReady`, and the table's `Curtain`
/// comes back as one (#256). The two halves of a handshake that, missing,
/// holds every game behind the engine's whole wait. Before its ready the
/// seat says a person answers it (`SeatMind`), for the game's record, or
/// the fake table lets it in to nothing.
#[test]
fn a_seat_that_says_it_is_ready_is_told_the_table_is_open() {
    let port = spawn_table();
    let mut host = NetworkHost::connect(ticket(port)).expect("connect");
    poll_until(&mut host, "the first view", |m| !views(m).is_empty());
    host.ready();
    poll_until(&mut host, "the curtain", |m| {
        m.iter().any(|m| matches!(m, HostMessage::Curtain))
    });
}

/// The whole opening: roster, print table, hand, and the first question.
#[test]
fn a_networked_seat_is_dealt_in() {
    let port = spawn_table();
    let mut host = NetworkHost::connect(ticket(port)).expect("connect");
    let messages = poll_until(&mut host, "the opening", |m| {
        !statics(m).is_empty() && !views(m).is_empty() && !choices(m).is_empty()
    });

    let statics = statics(&messages);
    let statics = statics.first().expect("the roster arrived");
    assert_eq!(statics.view_version, baylee_view::VIEW_VERSION);
    assert_eq!(statics.your_seat, SEAT);
    assert_eq!(statics.seat_name(PlayerId::new(0)), "You");
    assert_eq!(statics.seat_name(PlayerId::new(1)), "House AI");
    assert!(statics.seats[1].is_ai);
    assert!(!statics.seats[0].is_ai);
    assert_eq!(
        statics.prints.len(),
        1,
        "without the print table a PrintRef names no card"
    );

    // The host believes the table about which chair this is.
    assert_eq!(host.seat(), SEAT);
    assert!(host.is_open());

    let view = views(&messages)[0];
    assert_eq!(view.hand.len(), 7);
    // The house has already answered the AI chair's mulligan (#257). A hand
    // of seven Islands is never a keep for its policy, so it takes until its
    // limit and keeps five.
    let them = &view.seats[1];
    assert_eq!(them.hand_count, 5);
    assert_eq!(
        them.hand_count + them.library_count,
        60,
        "the opponent's hand and library are counts and nothing else"
    );
    assert!(matches!(choices(&messages)[0], Pending::Mulligan { .. }));
}

/// An answer travels, and the table answers back.
#[test]
fn an_answer_reaches_the_table() {
    let port = spawn_table();
    let mut host = NetworkHost::connect(ticket(port)).expect("connect");
    let opening = poll_until(&mut host, "the first question", |m| !choices(m).is_empty());
    let seq_before = host.last_seq();
    assert!(matches!(choices(&opening)[0], Pending::Mulligan { .. }));

    host.submit(PlayerAction::MulliganKeep);
    let after = poll_until(&mut host, "the game to move", |m| !views(m).is_empty());
    assert!(
        !after
            .iter()
            .any(|m| matches!(m, HostMessage::Failed(reason) if !reason.is_empty())),
        "keeping an opening hand is legal: {after:#?}"
    );
    assert!(
        host.last_seq() > seq_before,
        "the table moved, so the client's place in the stream did too"
    );
}

/// An illegal answer comes back as a message a player can be shown, rather
/// than as a table that silently stops responding.
#[test]
fn a_refused_answer_is_reported() {
    let port = spawn_table();
    let mut host = NetworkHost::connect(ticket(port)).expect("connect");
    poll_until(&mut host, "the first question", |m| !choices(m).is_empty());

    // The opening choice is a mulligan; passing priority is not an answer.
    host.submit(PlayerAction::PassPriority);
    let after = poll_until(&mut host, "the refusal", |m| {
        m.iter().any(|m| matches!(m, HostMessage::Failed(_)))
    });
    assert!(
        after
            .iter()
            .any(|m| matches!(m, HostMessage::Failed(reason) if reason.contains("illegal"))),
        "{after:#?}"
    );
}

/// A dropped connection is recoverable: the seat comes back to the same game,
/// not to a new one.
#[test]
fn a_reconnect_returns_to_the_same_table() {
    let port = spawn_table();
    let mut host = NetworkHost::connect(ticket(port)).expect("connect");
    poll_until(&mut host, "the first question", |m| !choices(m).is_empty());
    host.submit(PlayerAction::MulliganKeep);
    poll_until(&mut host, "the game to move", |m| !views(m).is_empty());
    let seq = host.last_seq();
    assert!(seq > 0);

    host.reconnect().expect("redial");
    let back = poll_until(&mut host, "the table again", |m| {
        !statics(m).is_empty() && !views(m).is_empty()
    });
    assert_eq!(statics(&back)[0].your_seat, SEAT);
    assert!(
        host.last_seq() >= seq,
        "a reconnect never rewinds the client's place in the stream"
    );
    // Still the same game: the seat is not asked to mulligan a second time.
    assert!(
        !choices(&back)
            .iter()
            .any(|p| matches!(p, Pending::Mulligan { .. })),
        "{back:#?}"
    );
}

/// The networked host answers the report form's keyring with its ticket's
/// seat token (#314): at the table the lobby no longer holds it, and this
/// is the one place it still lives.
#[test]
fn the_networked_host_names_its_seat_token_to_the_report_form() {
    let host = NetworkHost::connect(ticket(1)).expect("connect");
    assert_eq!(host.seat_token(), Some("0123456789abcdef"));
}

/// A seat's socket is opened with a ticket bought with its seat token in a
/// header, and the token itself is in no address the client dials (#294).
#[test]
fn a_seat_socket_is_opened_with_a_ticket_and_never_with_its_token() {
    let (port, seen) = spawn_table_behind(Door::default());
    let mut host = NetworkHost::connect(ticket(port)).expect("connect");
    poll_until(&mut host, "the opening", |m| !views(m).is_empty());
    let seen = seen.lock().expect("witness");
    assert_eq!(
        seen.ticket_requests,
        vec![format!("Bearer {SEAT_TOKEN}")],
        "one ticket, bought with the seat token in Authorization"
    );
    assert_eq!(seen.upgrades.len(), 1, "{:?}", seen.upgrades);
    for url in &seen.upgrades {
        assert!(url.contains("ticket=tkt0"), "{url}");
        assert!(!url.contains(SEAT_TOKEN), "the seat token is in {url}");
        assert!(!url.contains("token="), "{url}");
    }
}

/// An upgrade refused as a stale ticket is dialled again with a fresh one,
/// and the player never sees the link go down: the owner's "the client
/// recovers without the player doing anything".
#[test]
fn a_stale_ticket_is_replaced_and_the_player_sees_nothing() {
    let (port, seen) = spawn_table_behind(Door {
        refuse_upgrades: 2,
        ticket_status: None,
    });
    let mut host = NetworkHost::connect(ticket(port)).expect("connect");
    let mut all = Vec::new();
    for _ in 0..WAIT_TRIES {
        all.extend(host.poll());
        assert_ne!(
            host.link(),
            LinkState::Down,
            "a stale ticket showed the player a lost link"
        );
        if !views(&all).is_empty() {
            break;
        }
        std::thread::sleep(WAIT_STEP);
    }
    assert!(!views(&all).is_empty(), "never dealt in: {all:#?}");
    assert_eq!(host.link(), LinkState::Up);
    let seen = seen.lock().expect("witness");
    assert_eq!(seen.ticket_requests.len(), 3, "a fresh ticket per attempt");
    let tickets: Vec<&str> = seen
        .upgrades
        .iter()
        .filter_map(|u| u.split("ticket=").nth(1))
        .map(|rest| rest.split('&').next().unwrap_or_default())
        .collect();
    assert_eq!(
        tickets,
        ["tkt0", "tkt1", "tkt2"],
        "never the same ticket twice"
    );
}

/// Past the retries, the dial gives up and the ordinary reconnect schedule
/// takes over: the link is down, and says so.
#[test]
fn the_stale_ticket_retries_run_out_into_the_ordinary_schedule() {
    let (port, seen) = spawn_table_behind(Door {
        refuse_upgrades: 1 + usize::from(baylee_client_core::wsticket::STALE_TICKET_RETRIES),
        ticket_status: None,
    });
    let mut host = NetworkHost::connect(ticket(port)).expect("connect");
    for _ in 0..WAIT_TRIES {
        host.poll();
        if host.link() == LinkState::Down {
            break;
        }
        std::thread::sleep(WAIT_STEP);
    }
    assert_eq!(host.link(), LinkState::Down);
    assert_eq!(seen.lock().expect("witness").ticket_requests.len(), 3);
    // And the schedule's redial buys a ticket of its own and gets in.
    host.reconnect().expect("redial");
    poll_until(&mut host, "the table after the schedule's redial", |m| {
        !views(m).is_empty()
    });
}

/// A seat token the gateway no longer takes (`401` on the ticket request)
/// is a link that is down, and no socket is dialled with nothing to show.
#[test]
fn a_refused_ticket_request_is_a_link_that_is_down() {
    let (port, seen) = spawn_table_behind(Door {
        refuse_upgrades: 0,
        ticket_status: Some(401),
    });
    let mut host = NetworkHost::connect(ticket(port)).expect("connect");
    for _ in 0..WAIT_TRIES {
        host.poll();
        if host.link() == LinkState::Down {
            break;
        }
        std::thread::sleep(WAIT_STEP);
    }
    assert_eq!(host.link(), LinkState::Down);
    let seen = seen.lock().expect("witness");
    assert_eq!(seen.ticket_requests.len(), 1, "asked once, not in a loop");
    assert!(seen.upgrades.is_empty(), "no socket without a ticket");
}

/// Every reconnect buys a fresh ticket: the last one was spent on the
/// socket that went away.
#[test]
fn a_reconnect_buys_a_fresh_ticket() {
    let (port, seen) = spawn_table_behind(Door::default());
    let mut host = NetworkHost::connect(ticket(port)).expect("connect");
    poll_until(&mut host, "the first question", |m| !choices(m).is_empty());
    host.reconnect().expect("redial");
    poll_until(&mut host, "the table again", |m| !statics(m).is_empty());
    let seen = seen.lock().expect("witness");
    assert_eq!(seen.ticket_requests.len(), 2);
    assert!(
        seen.upgrades[0].contains("ticket=tkt0"),
        "{:?}",
        seen.upgrades
    );
    assert!(
        seen.upgrades[1].contains("ticket=tkt1"),
        "{:?}",
        seen.upgrades
    );
}

/// A real Session sends Ward through websocket framing and the native host;
/// the same client interaction model used by the buttons answers it.
#[test]
fn life_ward_payment_round_trips_from_session_through_the_client() {
    use baylee_client_core::interaction::Interaction;
    use baylee_engine::choice::YesNoPrompt;

    let entry = |name: &str| DeckEntry {
        card: baylee_cards::generated::ALL
            .iter()
            .find(|(_, def)| def.name() == name)
            .unwrap()
            .1
            .index,
        print: PrintRef::new(0),
    };
    for pay in [false, true] {
        let mut preset = duel_preset();
        preset.seats[0].starting_hand = Some(vec![entry("Swords to Plowshares")]);
        preset.seats[0].starting_battlefield = vec![entry("Plains")];
        preset.seats[1].starting_battlefield = vec![entry("Phyrexian Fleshgorger")];
        let (port, _) = spawn_table_with_preset(Door::default(), preset);
        let mut host = NetworkHost::connect(ticket(port)).expect("connect");
        let mut cast = false;
        let mut answered = false;
        let mut finished = false;
        for _ in 0..40 {
            let messages = poll_until(&mut host, "a Ward game choice", |m| !choices(m).is_empty());
            assert!(
                !messages.iter().any(|m| matches!(m, HostMessage::Failed(_))),
                "{messages:?}"
            );
            if answered {
                let view = views(&messages).last().copied().expect("updated view");
                if view.stack.is_empty() {
                    assert_eq!(view.seats[0].life, if pay { 13 } else { 20 });
                    let flesh = view.battlefield.iter().any(|object| {
                        object
                            .card
                            .as_ref()
                            .is_some_and(|c| c.index == entry("Phyrexian Fleshgorger").card)
                    });
                    assert_eq!(flesh, !pay);
                    finished = true;
                    break;
                }
            }
            let pending = (*choices(&messages).last().unwrap()).clone();
            let action = match &pending {
                Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
                Pending::Priority { legal, .. } if !cast && !legal.castable.is_empty() => {
                    cast = true;
                    PlayerAction::CastSpell {
                        card: legal.castable[0],
                    }
                }
                Pending::Priority { legal, .. } if !cast && !legal.mana_abilities.is_empty() => {
                    PlayerAction::ActivateManaAbility {
                        source: legal.mana_abilities[0],
                    }
                }
                Pending::Priority { .. } => PlayerAction::PassPriority,
                Pending::ChooseTargets { options, .. } => PlayerAction::ChooseObjects {
                    objects: vec![options[0]],
                },
                Pending::YesNo {
                    player,
                    prompt: YesNoPrompt::PayLife { amount: 7 },
                    ..
                } => {
                    assert_eq!(*player, SEAT);
                    assert!(!answered);
                    answered = true;
                    Interaction::new(pending.clone(), SEAT)
                        .answer_yes_no(pay)
                        .unwrap()
                }
                other => panic!("unexpected choice: {other:?}"),
            };
            host.submit(action);
        }
        assert!(
            cast && answered && finished,
            "the payment and its result must actually be reached"
        );
    }
}
