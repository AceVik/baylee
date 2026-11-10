//! What one caller may hold of the gateway: rooms, sockets, stored decks,
//! the memory a picture or a first frame costs.
//!
//! The gateway is one process for everybody, so anything a single caller can
//! grow without end is a way to take it from everybody else. Each test here
//! pushes one such thing past what a real client ever needs and checks that
//! the gateway says no, or lets go, before it costs anybody else anything.

#![allow(clippy::missing_docs_in_private_items)]

use crate::common;

use baylee_protocol::v1::{self, Envelope};
use common::{http, http_bytes, json_field, lobby_url, login, spawn_gateway, spawn_gateway_with};
use futures_util::{SinkExt, StreamExt};
use prost::Message as _;
use std::time::Duration;
use tokio_tungstenite::tungstenite::Message;

/// Whether `socket` is closed within `within`, whatever it says on the way.
async fn closes(socket: &mut common::Socket, within: Duration) -> bool {
    tokio::time::timeout(within, async {
        loop {
            match socket.next().await {
                Some(Ok(Message::Close(_)) | Err(_)) | None => return,
                Some(Ok(_)) => {}
            }
        }
    })
    .await
    .is_ok()
}

/// A first frame carrying a wrong secret and a name as long as `bytes`.
fn hello_of(bytes: usize) -> Vec<u8> {
    Envelope {
        msg: Some(v1::envelope::Msg::AgentHello(v1::AgentHello {
            token: "not the secret".into(),
            name: "x".repeat(bytes),
            capacity: 1,
            protocol_version: baylee_protocol::PROTOCOL_VERSION,
        })),
    }
    .encode_to_vec()
}

/// A stranger's first frame on the agent's link is read only as far as an
/// agent's ever goes. Nobody has proved anything yet, so nothing the
/// transport would have taken may be held, and no name of its choosing
/// reaches the log.
#[tokio::test(flavor = "multi_thread")]
async fn a_stranger_cannot_send_the_agent_link_a_large_first_frame() {
    let gw = spawn_gateway_with("bounds_agent", &[("RUST_LOG", "warn".into())]);
    let url = format!("ws://127.0.0.1:{}/agent/ws", gw.port);
    let mut ws = common::dial(&url).await.expect("the agent's door");
    let _ = ws.send(Message::Binary(hello_of(1 << 20).into())).await;
    assert!(
        closes(&mut ws, Duration::from_secs(9)).await,
        "the link stayed open"
    );
    assert!(
        !gw.logs().contains("xxxxxxxx"),
        "a stranger's name reached the log whole"
    );
}

/// A hello a stranger sends that fits still leaves no name of its choosing
/// in the log, because nothing it said has been believed yet.
#[tokio::test(flavor = "multi_thread")]
async fn a_wrong_secret_writes_no_stranger_s_name_to_the_log() {
    let gw = spawn_gateway_with("bounds_agent_name", &[("RUST_LOG", "warn".into())]);
    let url = format!("ws://127.0.0.1:{}/agent/ws", gw.port);
    let mut ws = common::dial(&url).await.expect("the agent's door");
    ws.send(Message::Binary(hello_of(64).into()))
        .await
        .expect("sent");
    assert!(closes(&mut ws, Duration::from_secs(9)).await);
    let logs = gw.logs();
    assert!(
        logs.contains("agent presented the wrong token"),
        "the refusal is logged: {logs}"
    );
    assert!(!logs.contains("xxxxxxxx"), "{logs}");
}

/// An address holds only so many sockets before their hello: what they send
/// is read before anything about them is known. Behind a trusted proxy, so
/// the address is the one it names and not this machine, which is exempt.
#[tokio::test(flavor = "multi_thread")]
async fn an_address_holds_only_so_many_sockets_before_their_hello() {
    use tokio_tungstenite::tungstenite::client::IntoClientRequest as _;
    let gw = spawn_gateway_with(
        "bounds_hellos",
        &[("BAYLEE_TRUSTED_PROXIES", "127.0.0.1".into())],
    );
    let dial = |forwarded: &'static str| {
        let mut request = format!("ws://127.0.0.1:{}/engine/ws", gw.port)
            .into_client_request()
            .expect("a request");
        request
            .headers_mut()
            .insert("x-forwarded-for", forwarded.parse().expect("a value"));
        tokio_tungstenite::connect_async(request)
    };
    let mut held = Vec::new();
    for _ in 0..16 {
        held.push(dial("203.0.113.9").await.expect("a place").0);
    }
    match dial("203.0.113.9").await {
        Err(tokio_tungstenite::tungstenite::Error::Http(answer)) => {
            assert_eq!(answer.status().as_u16(), 429);
        }
        other => panic!("the seventeenth: {:?}", other.map(|_| "opened")),
    }
    held.push(dial("203.0.113.10").await.expect("another address").0);
}

/// An address has only so many large bodies on their way at once, however
/// many connections it opens: each may be megabytes, read before anything
/// about the sender is known. A small one needs no place.
#[test]
fn an_address_has_only_so_many_large_bodies_on_their_way() {
    use std::io::{Read as _, Write as _};
    let gw = spawn_gateway_with(
        "bounds_bodies",
        &[("BAYLEE_TRUSTED_PROXIES", "127.0.0.1".into())],
    );
    let start = |length: usize| {
        let mut stream = std::net::TcpStream::connect(("127.0.0.1", gw.port)).expect("connected");
        write!(
            stream,
            "POST /auth/guest HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\n\
             X-Forwarded-For: 203.0.113.9\r\nContent-Length: {length}\r\n\r\n"
        )
        .expect("written");
        stream
    };
    let held: Vec<_> = (0..16).map(|_| start(1 << 20)).collect();
    std::thread::sleep(Duration::from_millis(300));
    let mut seventeenth = start(1 << 20);
    seventeenth
        .set_read_timeout(Some(Duration::from_secs(10)))
        .expect("a timeout");
    let mut answer = [0_u8; 12];
    seventeenth.read_exact(&mut answer).expect("an answer");
    assert_eq!(&answer, b"HTTP/1.1 429");
    let mut small = start(2);
    small.write_all(b"{}").expect("written");
    small
        .set_read_timeout(Some(Duration::from_secs(10)))
        .expect("a timeout");
    small.read_exact(&mut answer).expect("an answer");
    assert_ne!(&answer, b"HTTP/1.1 429", "a small body needs no place");
    drop(held);
}

/// A seat agent has one hello window, not one per frame it cares to send.
#[tokio::test(flavor = "multi_thread")]
async fn a_stranger_cannot_keep_the_seat_agent_door_open_with_frames() {
    let gw = spawn_gateway("bounds_seathost");
    let url = format!("ws://127.0.0.1:{}/seathost/ws", gw.port);
    let mut ws = common::dial(&url).await.expect("the seat agent's door");
    let deadline = tokio::time::Instant::now() + Duration::from_secs(14);
    let mut open = true;
    while tokio::time::Instant::now() < deadline {
        if ws
            .send(Message::Binary(vec![0_u8; 4].into()))
            .await
            .is_err()
        {
            open = false;
            break;
        }
        if closes(&mut ws, Duration::from_secs(3)).await {
            open = false;
            break;
        }
    }
    assert!(!open, "fourteen seconds past a ten-second hello window");
}

/// The lobby's feed reads nothing from a client, so it has no use for a
/// large frame.
#[tokio::test(flavor = "multi_thread")]
async fn the_lobby_feed_takes_no_large_frame() {
    let gw = spawn_gateway("bounds_lobby_frame");
    let session = login(gw.port, "framer", "Framer");
    let url = lobby_url(gw.port, &session, "");
    let mut ws = common::dial(&url).await.expect("the lobby feed");
    let _ = ws.send(Message::Binary(vec![0_u8; 1 << 20].into())).await;
    assert!(
        closes(&mut ws, Duration::from_secs(5)).await,
        "a megabyte on a socket that reads nothing"
    );
}

/// A seat's frames are counted by weight as well as by number: frames at the
/// size limit and inside the frame allowance would otherwise be megabytes a
/// second into the queue to its engine, in the process every table shares.
#[tokio::test(flavor = "multi_thread")]
async fn a_seat_cannot_send_megabytes_a_second() {
    let gw = spawn_gateway("bounds_seat_bytes");
    let port = gw.port;
    let _agent = common::attach_agent(&gw).await;
    let token = login(port, "heavy", "Heavy");
    let (status, body) = http(
        port,
        "POST",
        "/decks",
        Some(&token),
        r#"{"name":"d","cards":["40 Forest","20 Swamp"]}"#,
    );
    assert_eq!(status, 200, "{body}");
    let create = format!(
        r#"{{"deck_id":"{}","mode":"ai"}}"#,
        json_field(&body, "deck_id")
    );
    let (status, body) = http(port, "POST", "/lobby/games", Some(&token), &create);
    assert_eq!(status, 200, "{body}");
    let mut ws = common::dial_seat(
        port,
        json_field(&body, "game_id"),
        json_field(&body, "seat_token"),
    )
    .await;
    // Far under the frame allowance's 512 at once, and over 12 MiB.
    let heavy = Message::Binary(vec![0_u8; 60 * 1024].into());
    for _ in 0..200 {
        if ws.feed(heavy.clone()).await.is_err() {
            break;
        }
    }
    let _ = ws.flush().await;
    let closed = tokio::time::timeout(common::WAIT_BUDGET, async {
        while let Some(frame) = ws.next().await {
            match frame {
                Ok(Message::Close(frame)) => return frame,
                Ok(_) => {}
                Err(_) => break,
            }
        }
        None
    })
    .await
    .expect("a heavy seat was still open");
    let closed = closed.expect("closed saying why");
    assert_eq!(u16::from(closed.code), 1008, "{closed:?}");
}

/// A room's password is guessed only so often by one account, and once that
/// is spent even the right one has to wait: a limit that only changed the
/// answer to a wrong guess would not stop anybody guessing.
#[test]
fn a_room_s_password_is_guessed_only_so_often() {
    let gw = spawn_gateway("bounds_password");
    let host = login(gw.port, "locker", "Locker");
    let (status, body) = http(
        gw.port,
        "POST",
        "/lobby/games",
        Some(&host),
        r#"{"mode":"open","seats":3,"password":"right"}"#,
    );
    assert_eq!(status, 200, "{body}");
    let path = format!("/lobby/games/{}/join", json_field(&body, "game_id"));
    let guesser = login(gw.port, "guesser", "Guesser");
    let guess = |password: &str, who: &str| {
        let body = format!(r#"{{"password":"{password}"}}"#);
        http(gw.port, "POST", &path, Some(who), &body).0
    };
    for _ in 0..8 {
        assert_eq!(guess("wrong", &guesser), 403);
    }
    assert_eq!(guess("wrong", &guesser), 429);
    assert_eq!(guess("right", &guesser), 429, "spent is spent");
    let friend = login(gw.port, "friend", "Friend");
    assert_eq!(
        guess("right", &friend),
        200,
        "another account is not held up"
    );
}

/// Each spectator that opens costs its engine a whole snapshot, so one
/// account's tickets to watch are limited, however briefly it stays.
#[tokio::test(flavor = "multi_thread")]
async fn an_account_buys_only_so_many_tickets_to_watch() {
    let gw = spawn_gateway("bounds_watch");
    let port = gw.port;
    let _agent = common::attach_agent(&gw).await;
    let player = login(port, "watched", "Watched");
    let (status, body) = http(
        port,
        "POST",
        "/decks",
        Some(&player),
        r#"{"name":"d","cards":["40 Forest","20 Swamp"]}"#,
    );
    assert_eq!(status, 200, "{body}");
    let create = format!(
        r#"{{"deck_id":"{}","mode":"ai"}}"#,
        json_field(&body, "deck_id")
    );
    let (status, body) = http(port, "POST", "/lobby/games", Some(&player), &create);
    assert_eq!(status, 200, "{body}");
    let ask = format!(
        r#"{{"socket":"watch","game":"{}"}}"#,
        json_field(&body, "game_id")
    );
    let watcher = login(port, "watcher", "Watcher");
    let mut bought = 0;
    let status = loop {
        let (status, _) = common::ask_ticket(port, &watcher, &ask);
        if status != 200 {
            break status;
        }
        bought += 1;
        assert!(bought <= 100, "{bought} tickets and still going");
    };
    assert_eq!(status, 429);
    assert!(bought >= 5, "a spectator who reconnects is not stopped");
}

/// A name everybody in the lobby reads carries nothing that could dress a
/// line up as another.
#[test]
fn a_room_and_a_deck_are_named_without_control_characters() {
    let gw = spawn_gateway("bounds_names");
    let session = login(gw.port, "namer", "Namer");
    let (status, body) = http(
        gw.port,
        "POST",
        "/lobby/games",
        Some(&session),
        &serde_json::json!({ "mode": "open", "name": "Room\n\u{202e}one" }).to_string(),
    );
    assert_eq!(status, 200, "{body}");
    let (status, listing) = http(gw.port, "GET", "/lobby/games", Some(&session), "");
    assert_eq!(status, 200, "{listing}");
    assert!(listing.contains(r#""name":"Roomone""#), "{listing}");
    let (status, body) = http(
        gw.port,
        "POST",
        "/decks",
        Some(&session),
        r#"{"name":"Deck\none","cards":["1 Forest"]}"#,
    );
    assert_eq!(status, 400, "{body}");
}

/// One account cannot open waiting rooms without end: every one of them is
/// memory and a row in everybody's listing.
#[test]
fn an_account_opens_only_so_many_waiting_rooms() {
    let gw = spawn_gateway("bounds_rooms");
    let session = login(gw.port, "roomer", "Roomer");
    let open = |port| {
        http(
            port,
            "POST",
            "/lobby/games",
            Some(&session),
            r#"{"mode":"open"}"#,
        )
    };
    let mut opened = 0;
    let refused = loop {
        let (status, body) = open(gw.port);
        if status != 200 {
            break (status, body);
        }
        opened += 1;
        assert!(opened <= 64, "{opened} rooms and still going");
    };
    assert_eq!(refused.0, 429, "{}", refused.1);
    assert!(opened >= 2, "more than one room stays allowed");
    // The bound is per account.
    let other = login(gw.port, "other", "Other");
    let (status, body) = http(
        gw.port,
        "POST",
        "/lobby/games",
        Some(&other),
        r#"{"mode":"open"}"#,
    );
    assert_eq!(status, 200, "{body}");
}

/// One account cannot hold sockets without end: each is read and
/// re-authorised for as long as it stays open.
#[tokio::test(flavor = "multi_thread")]
async fn an_account_holds_only_so_many_sockets() {
    let gw = spawn_gateway("bounds_sockets");
    let session = login(gw.port, "socketer", "Socketer");
    let mut held = Vec::new();
    let mut refused = false;
    for _ in 0..64 {
        let url = lobby_url(gw.port, &session, "");
        match tokio_tungstenite::connect_async(url.as_str()).await {
            Ok((socket, _)) => held.push(socket),
            Err(tokio_tungstenite::tungstenite::Error::Http(answer)) => {
                assert_eq!(answer.status().as_u16(), 429);
                refused = true;
                break;
            }
            Err(e) => panic!("{e}"),
        }
    }
    assert!(refused, "{} lobby sockets and still going", held.len());
    assert!(held.len() >= 4, "a player's own windows stay allowed");
    // A closed socket gives its place back, or a client that reconnects
    // would lock itself out.
    let mut one = held.pop().expect("one held");
    let _ = one.close(None).await;
    drop(one);
    let mut back = false;
    for _ in 0..50 {
        let url = lobby_url(gw.port, &session, "");
        if let Ok((socket, _)) = tokio_tungstenite::connect_async(url.as_str()).await {
            held.push(socket);
            back = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(back, "the place of a closed socket came back");
}

/// CRC-32 as a PNG chunk wants it.
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let low = crc & 1;
            crc >>= 1;
            if low != 0 {
                crc ^= 0xEDB8_8320;
            }
        }
    }
    !crc
}

/// A grey PNG of `side` × `side` that says so in its header and compresses
/// to almost nothing, written row by row so the test holds none of it.
fn a_bomb(side: u32) -> Vec<u8> {
    use std::io::Write as _;
    let chunk = |out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]| {
        out.extend_from_slice(&u32::try_from(data.len()).expect("fits").to_be_bytes());
        let start = out.len();
        out.extend_from_slice(kind);
        out.extend_from_slice(data);
        let crc = crc32(&out[start..]);
        out.extend_from_slice(&crc.to_be_bytes());
    };
    let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::best());
    let row = vec![0_u8; side as usize + 1];
    for _ in 0..side {
        z.write_all(&row).expect("compressed");
    }
    let idat = z.finish().expect("compressed");
    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&side.to_be_bytes());
    ihdr.extend_from_slice(&side.to_be_bytes());
    ihdr.extend_from_slice(&[8, 0, 0, 0, 0]);
    chunk(&mut png, b"IHDR", &ihdr);
    chunk(&mut png, b"IDAT", &idat);
    chunk(&mut png, b"IEND", &[]);
    png
}

/// The most resident memory the gateway has held, in kibibytes.
fn peak_kib(gw: &common::Gateway) -> u64 {
    let status = std::fs::read_to_string(format!("/proc/{}/status", gw.pid())).unwrap_or_default();
    status
        .lines()
        .find_map(|line| line.strip_prefix("VmHWM:"))
        .and_then(|rest| rest.split_whitespace().next())
        .and_then(|kib| kib.parse().ok())
        .unwrap_or(0)
}

/// A picture is refused by what its header says it would cost, before it is
/// decoded: a few hundred kilobytes must not turn into gigabytes of the
/// process everybody plays on.
#[test]
fn a_picture_that_would_unpack_to_gigabytes_is_refused_unread() {
    let dir = std::env::temp_dir().join(format!("baylee-bomb-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let gw = spawn_gateway_with(
        "bounds_bomb",
        &[("BAYLEE_DECK_IMAGE_PATH", dir.to_string_lossy().into_owned())],
    );
    let session = login(gw.port, "bomber", "Bomber");
    let bomb = a_bomb(20_000);
    assert!(bomb.len() < 2 << 20, "{} bytes on the wire", bomb.len());
    let before = peak_kib(&gw);
    let started = std::time::Instant::now();
    let (status, body) = http_bytes(
        gw.port,
        "POST",
        "/images?kind=sleeve",
        Some(&session),
        "image/png",
        &bomb,
    );
    let grew = peak_kib(&gw).saturating_sub(before);
    eprintln!(
        "bomb: status {status}, {} bytes sent, peak grew {} MiB in {:?}",
        bomb.len(),
        grew / 1024,
        started.elapsed()
    );
    assert_eq!(status, 413, "{}", String::from_utf8_lossy(&body));
    assert!(grew < 64 * 1024, "the peak grew {} MiB", grew / 1024);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Uploads are decoded on the blocking pool, so each costs CPU and memory
/// whether or not anything is kept: one account has a budget of them.
#[test]
fn an_account_uploads_only_so_many_pictures() {
    let dir = std::env::temp_dir().join(format!("baylee-uploads-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let gw = spawn_gateway_with(
        "bounds_uploads",
        &[("BAYLEE_DECK_IMAGE_PATH", dir.to_string_lossy().into_owned())],
    );
    let session = login(gw.port, "uploader", "Uploader");
    let picture = {
        let mut out = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(image::RgbImage::new(8, 8))
            .write_to(&mut out, image::ImageFormat::Png)
            .expect("a png");
        out.into_inner()
    };
    let mut sent = 0;
    let status = loop {
        let (status, _) = http_bytes(
            gw.port,
            "POST",
            "/images?kind=sleeve",
            Some(&session),
            "image/png",
            &picture,
        );
        if status != 200 {
            break status;
        }
        sent += 1;
        assert!(sent <= 200, "{sent} uploads and still going");
    };
    assert_eq!(status, 429);
    assert!(sent >= 10, "a player choosing a sleeve is not stopped");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A deck's free text is bounded by what a deck's text is, not by the
/// transport's two megabytes: every save of it is kept.
#[test]
fn a_deck_s_free_text_is_bounded() {
    let gw = spawn_gateway("bounds_deck_text");
    let session = login(gw.port, "writer", "Writer");
    let deck =
        |extra: &str| format!(r#"{{"name":"Long","cards":["1 Forest"],{extra}"sideboard":[]}}"#);
    let long = "x".repeat(1 << 20);
    for field in ["description", "format", "summary"] {
        let body = deck(&format!(r#""{field}":"{long}","#));
        let (status, answer) = http(gw.port, "POST", "/decks", Some(&session), &body);
        assert_eq!(status, 400, "{field}: {answer}");
    }
    let (status, answer) = http(
        gw.port,
        "POST",
        "/decks",
        Some(&session),
        &deck(r#""description":"A short, honest description.","#),
    );
    assert_eq!(status, 200, "{answer}");
}

/// One account cannot store decks without end.
#[test]
fn an_account_stores_only_so_many_decks() {
    let gw = spawn_gateway("bounds_decks");
    let session = login(gw.port, "hoarder", "Hoarder");
    let body = r#"{"name":"Deck","cards":["1 Forest"]}"#;
    let (status, answer) = http(gw.port, "POST", "/decks", Some(&session), body);
    assert_eq!(status, 200, "{answer}");
    let deck = json_field(&answer, "deck_id").to_string();
    // A shelf filled by hand: thousands of saves are not what this is about.
    gw.sql(&format!(
        "INSERT INTO deck (account_id, kind, name, format, version, cards, sideboard, commanders,
             updated_at, offered)
         SELECT d.account_id, d.kind, d.name, d.format, 1, d.cards, d.sideboard, d.commanders,
             d.updated_at, true
         FROM deck d, generate_series(1, 1200) WHERE d.id = '{deck}'"
    ));
    let (status, answer) = http(gw.port, "POST", "/decks", Some(&session), body);
    assert_eq!(status, 409, "{answer}");
    let (status, answer) = http(
        gw.port,
        "POST",
        &format!("/decks/{deck}/copy"),
        Some(&session),
        "{}",
    );
    assert_eq!(status, 409, "{answer}");
    // What it has stays its own to change.
    let (status, answer) = http(
        gw.port,
        "PUT",
        &format!("/decks/{deck}"),
        Some(&session),
        r#"{"name":"Renamed","cards":["1 Forest"]}"#,
    );
    assert_eq!(status, 200, "{answer}");
}
