//! The lobby feed's door (#294): a ticket bought with the session in a
//! header, the socket dialled with the ticket and never with the session,
//! and a session the gateway no longer knows sent back to sign in.

#[allow(clippy::wildcard_imports)]
use super::*;

use baylee_client_core::lobby::KeptGuest;
use std::io::{Read as _, Write as _};

/// What the fake gateway saw.
#[derive(Default)]
struct Seen {
    /// `Authorization` of every `POST /ws-ticket`.
    ticket_requests: Vec<String>,
    /// The path and query of every socket upgrade.
    upgrades: Vec<String>,
}

type Witness = Arc<Mutex<Seen>>;

/// A gateway that answers `POST /ws-ticket` with `ticket_status` (a ticket
/// on 200), upgrades a socket that shows it and sends one empty listing, and
/// answers everything else `404` — never `401`, so the only refusal a test
/// can see is the ticket route's.
#[allow(clippy::result_large_err)] // tungstenite's handshake callback, as it is declared
fn fake_gateway(ticket_status: u16) -> (String, Witness) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let seen = Witness::default();
    let witness = seen.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { return };
            let mut head = [0u8; 4];
            if stream.peek(&mut head).is_err() {
                continue;
            }
            if &head == b"GET " && peek_upgrade(&stream) {
                let seen = seen.clone();
                let Ok(mut ws) = tungstenite::accept_hdr(
                    stream,
                    move |request: &tungstenite::handshake::server::Request, response| {
                        seen.lock()
                            .expect("witness")
                            .upgrades
                            .push(request.uri().to_string());
                        Ok(response)
                    },
                ) else {
                    continue;
                };
                let listing = r#"{"games":[],"total":0,"offset":0,"limit":20}"#;
                let _ = ws.send(tungstenite::Message::Text(listing.into()));
                // Held open until the client hangs up.
                while ws.read().is_ok() {}
                continue;
            }
            let request = read_request(&mut stream);
            let (status, body) = if request.starts_with("POST /ws-ticket ") {
                let bearer = request
                    .lines()
                    .find_map(|l| {
                        let (name, value) = l.split_once(':')?;
                        name.eq_ignore_ascii_case("authorization")
                            .then(|| value.trim().to_string())
                    })
                    .unwrap_or_default();
                seen.lock().expect("witness").ticket_requests.push(bearer);
                if ticket_status == 200 {
                    (200, r#"{"ticket":"tkt-lobby","expires_in":45}"#)
                } else {
                    (ticket_status, r#"{"error":"invalid or expired token"}"#)
                }
            } else {
                (404, r#"{"error":"not here"}"#)
            };
            let reply = format!(
                "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(reply.as_bytes());
        }
    });
    (format!("http://127.0.0.1:{port}"), witness)
}

/// Whether the request waiting on `stream` is a websocket upgrade.
fn peek_upgrade(stream: &std::net::TcpStream) -> bool {
    let mut buf = [0u8; 2048];
    let n = stream.peek(&mut buf).unwrap_or(0);
    String::from_utf8_lossy(&buf[..n])
        .to_ascii_lowercase()
        .contains("upgrade: websocket")
}

/// Reads one request's head and body.
fn read_request(stream: &mut std::net::TcpStream) -> String {
    let mut raw = Vec::new();
    let mut buf = [0u8; 1024];
    while let Ok(n) = stream.read(&mut buf) {
        if n == 0 {
            break;
        }
        raw.extend_from_slice(&buf[..n]);
        let text = String::from_utf8_lossy(&raw);
        if let Some(end) = text.find("\r\n\r\n") {
            let length = text[..end]
                .lines()
                .find_map(|l| {
                    l.to_ascii_lowercase()
                        .strip_prefix("content-length:")
                        .map(|v| v.trim().parse::<usize>().unwrap_or(0))
                })
                .unwrap_or(0);
            if raw.len() >= end + 4 + length {
                break;
            }
        }
    }
    String::from_utf8_lossy(&raw).into_owned()
}

/// A guest signed in to `gateway`, at the tables.
fn at_the_tables(gateway: &str) -> App {
    let mut app = headless();
    app.world_mut().resource_mut::<LobbyState>().gateway = gateway.to_string();
    app.world()
        .resource::<Mailbox>()
        .0
        .lock()
        .expect("mailbox")
        .push(Reply::Event(LobbyEvent::GuestIn(KeptGuest {
            token: "5e55105e55105e55".to_string(),
            handle: "Casper#0007".to_string(),
        })));
    app.update();
    settle(&mut app);
    assert_eq!(
        *app.world().resource::<LobbyState>().lobby.screen(),
        Screen::Table
    );
    app
}

/// The socket of the session at the tables closes: what the feed does then,
/// in the frame before `poll`.
fn the_socket_closes(app: &mut App) {
    let world = app.world_mut();
    let epoch = world.resource::<LobbyState>().gateway_epoch;
    world.resource_scope(|world, mut feed: Mut<super::super::feed::Feed>| {
        feed.epoch = epoch;
        super::super::feed::give_up(
            &mut feed,
            world.resource::<LobbyState>(),
            world.resource::<Mailbox>(),
        );
    });
}

/// A socket lost under a live session is a lost gateway, said at once, in
/// the session's own epoch. Read from the mailbox: the next reply to arrive
/// would say the gateway is back.
#[test]
fn a_socket_lost_under_a_session_says_the_gateway_is_gone() {
    let mut app = at_the_tables("http://127.0.0.1:1");
    the_socket_closes(&mut app);
    let epoch = app.world().resource::<LobbyState>().gateway_epoch;
    let mail = app.world().resource::<Mailbox>().0.lock().expect("mailbox");
    assert!(mail.iter().any(|reply| matches!(
        reply,
        Reply::Remote(said, lost)
            if *said == epoch && matches!(**lost, Reply::Event(LobbyEvent::GatewayLost))
    )));
}

/// A gateway closes the socket of the account it deletes, beside the answer
/// that says so. The answer can be in the mailbox first, and the close then
/// stands behind it: the gateway has lost nobody (seen live, 06.11.2026).
#[test]
fn the_socket_closed_with_a_deleted_account_is_no_lost_gateway() {
    let mut app = at_the_tables("http://127.0.0.1:1");
    app.world()
        .resource::<Mailbox>()
        .0
        .lock()
        .expect("mailbox")
        .push(Reply::Event(LobbyEvent::AccountDeleted));
    the_socket_closes(&mut app);
    app.update();
    let lobby = &app.world().resource::<LobbyState>().lobby;
    assert_eq!(lobby.token(), None);
    assert!(!lobby.unreachable());
}

/// Runs frames until `done` or the budget runs out.
fn frames_until(app: &mut App, what: &str, done: impl Fn(&mut App) -> bool) {
    for _ in 0..600 {
        app.update();
        if done(app) {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    panic!("waited six seconds for {what}");
}

#[test]
fn the_feed_buys_a_ticket_with_the_session_and_dials_with_the_ticket() {
    let (gateway, seen) = fake_gateway(200);
    let mut app = at_the_tables(&gateway);
    frames_until(&mut app, "the feed to deliver", |app| {
        app.world().resource::<super::super::feed::Feed>().live()
    });
    let seen = seen.lock().expect("witness");
    assert_eq!(seen.ticket_requests, ["Bearer 5e55105e55105e55"]);
    assert_eq!(seen.upgrades.len(), 1, "{:?}", seen.upgrades);
    let url = &seen.upgrades[0];
    assert!(url.starts_with("/lobby/ws?ticket=tkt-lobby&q="), "{url}");
    assert!(!url.contains("5e55105e55105e55"), "the session is in {url}");
    assert!(!url.contains("token="), "{url}");
}

/// A `401` on the ticket request is the session gone: the player is sent
/// back to sign in, as after any other signed `401`.
#[test]
fn a_session_the_gateway_forgot_is_sent_back_to_sign_in() {
    let (gateway, seen) = fake_gateway(401);
    let mut app = at_the_tables(&gateway);
    frames_until(&mut app, "the lobby to sign the guest out", |app| {
        app.world().resource::<LobbyState>().lobby.token().is_none()
    });
    let seen = seen.lock().expect("witness");
    assert!(
        !seen.ticket_requests.is_empty(),
        "the sign-out came from somewhere other than the ticket request"
    );
    assert!(seen.upgrades.is_empty(), "no socket without a ticket");
}
