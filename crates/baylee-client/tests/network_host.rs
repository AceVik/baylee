//! The networked host, against a real websocket server.
//!
//! Everything else about a duel is testable as arithmetic; the transport is
//! not. So this spins up a socket, puts a real [`Session`] behind it, and
//! plays through [`NetworkHost`] exactly as the gateway would be played
//! through — including the reconnect, which is the part that only ever runs
//! when something has already gone wrong.

#![allow(clippy::missing_docs_in_private_items)]

use baylee_client::host::{DuelHost, HostMessage};
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

/// Starts a table on a free port and returns it.
///
/// Connections are served one at a time on purpose: the reconnect test needs
/// the *same* session to still be there when the second socket arrives, which
/// is exactly what a gateway guarantees and a fresh game would not.
fn spawn_table() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    listener.set_nonblocking(true).expect("nonblocking");
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        runtime.block_on(async move {
            let listener = tokio::net::TcpListener::from_std(listener).expect("listener");
            let mut session = Session::new(&duel_preset()).expect("session");
            session.describe(GAME.to_string(), names());
            while let Ok((stream, _)) = listener.accept().await {
                serve(stream, &mut session).await;
            }
        });
    });
    port
}

/// One connection: the opening payload, then answers until the socket closes.
async fn serve(stream: tokio::net::TcpStream, session: &mut Session) {
    let Ok(mut ws) = tokio_tungstenite::accept_async(stream).await else {
        return;
    };
    let mut opening = vec![session.game_static_envelope(SEAT)];
    opening.extend(mine(session.pump()));
    for envelope in opening {
        if send(&mut ws, envelope).await.is_err() {
            return;
        }
    }
    while let Some(Ok(frame)) = ws.next().await {
        if !frame.is_binary() {
            continue;
        }
        let Ok(envelope) = Envelope::decode(frame.into_data()) else {
            continue;
        };
        let replies = match envelope.msg {
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
        seat_token: "0123456789abcdef".to_string(),
    }
}

/// Polls the host until the messages so far satisfy `done`.
///
/// A frame loop with a deadline, which is what the client itself does: the
/// host may never block, so waiting is the caller's job.
fn poll_until<F>(host: &mut NetworkHost, what: &str, mut done: F) -> Vec<HostMessage>
where
    F: FnMut(&[HostMessage]) -> bool,
{
    let mut all = Vec::new();
    for _ in 0..1000 {
        all.extend(host.poll());
        if done(&all) {
            return all;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    panic!("waited 10s for {what}; got {all:#?}");
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
            HostMessage::View(v) => Some(&**v),
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
    assert_eq!(view.seats[1].hand_count, 7);
    assert_eq!(
        view.seats[1].library_count, 53,
        "the opponent's library is a count and nothing else"
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
