//! A seat socket has a frame allowance (#284): one that floods is closed
//! with 1008, which says why, rather than having its frames dropped.
//!
//! The allowance is set well above what a real client sends, and every
//! other test in this suite plays through it. So this one only floods: ten
//! thousand frames at once, far past any allowance the gateway could run.
//!
//! The allowance is counted as the gateway reads, so it trips only while
//! the gateway reads faster than [`RATE`]. A full authorization check on
//! every frame (one database query each) held a loaded machine's reading
//! below that on 21.10.2026, and the socket stayed open past the wait.
//! Reading costs no query now, and what is asserted is the allowance's own
//! arithmetic on what the gateway says it read, not how fast it read.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use baylee_protocol::v1::{self, Envelope};
use common::{http, json_field, login, spawn_gateway, spawn_gateway_with};
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;

/// `seatrate::BURST`: the most frames a seat may send at once.
const BURST: u64 = 512;

/// `seatrate::RATE`: frames a second once the burst is spent.
const RATE: u64 = 240;

/// The gateway's log without its colours.
fn plain(logs: &str) -> String {
    let mut out = String::with_capacity(logs.len());
    let mut chars = logs.chars();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            for c in chars.by_ref() {
                if c == 'm' {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// The number `name=` says on the first line of the gateway's log that
/// carries `marker`, waiting up to [`common::WAIT_BUDGET`] for the line.
async fn logged(gw: &common::Gateway, marker: &str, name: &str) -> Option<u64> {
    for _ in 0..common::WAIT_TRIES {
        let logs = plain(&gw.logs());
        if let Some(line) = logs.lines().find(|line| line.contains(marker)) {
            let at = line.find(&format!(" {name}="))? + name.len() + 2;
            let digits: String = line[at..]
                .chars()
                .take_while(char::is_ascii_digit)
                .collect();
            return digits.parse().ok();
        }
        tokio::time::sleep(common::WAIT_STEP).await;
    }
    None
}

#[tokio::test]
async fn a_seat_that_floods_its_socket_is_closed_with_policy() {
    // The gateway says how many frames a seat socket had read when it
    // closes it, over its allowance or otherwise.
    let gw = spawn_gateway_with(
        "seat-rate",
        &[(
            "RUST_LOG",
            "baylee_gateway::routes::seat_socket=info".to_owned(),
        )],
    );
    let port = gw.port;
    let _agent = common::attach_agent(&gw).await;
    let token = login(port, "flooder", "flooder");
    let (status, body) = http(
        port,
        "POST",
        "/decks",
        Some(&token),
        "{\"name\":\"d\",\"cards\":[\"40 Forest\",\"20 Swamp\"]}",
    );
    assert_eq!(status, 200, "create deck: {body}");
    let create = format!(
        "{{\"deck_id\":\"{}\",\"mode\":\"ai\"}}",
        json_field(&body, "deck_id")
    );
    let (status, body) = http(port, "POST", "/lobby/games", Some(&token), &create);
    assert_eq!(status, 200, "a game against the house: {body}");
    let game_id = json_field(&body, "game_id");
    let seat_token = json_field(&body, "seat_token");

    // From before the dial: the gateway's allowance starts when its socket
    // opens, which is later, so this is the longer of the two.
    let dialled = std::time::Instant::now();
    let mut ws = common::dial_seat(port, game_id, seat_token).await;
    // Flood transport frames: a game envelope is reauthorized against the
    // database before the next recv, which can pace it below the rate limit.
    // Unsolicited pongs spend the same allowance without that unrelated work.
    let flood = Message::Pong(Vec::new().into());
    for _ in 0..10_000 {
        if ws.feed(flood.clone()).await.is_err() {
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
    .await;
    let elapsed = dialled.elapsed();
    // The most frames the allowance admits in `elapsed`, and the one it
    // refuses: the burst, the rate since, and one for the interval's
    // rounding (`1 s / RATE` in whole nanoseconds).
    let ceiling =
        BURST + 2 + (RATE * u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)).div_ceil(1000);
    let Ok(closed) = closed else {
        // Said by the gateway as the socket goes: how much it had read.
        drop(ws);
        let read = logged(&gw, "seat socket closed", "frames").await;
        panic!(
            "a flooding seat was still open after {elapsed:?}, the gateway having read {read:?} \
             frames; it closes one past {BURST} at once and {RATE} a second after (at most \
             {ceiling} by now), so {}",
            if read.is_some_and(|read| read > ceiling) {
                "the allowance was not enforced"
            } else {
                "the gateway never read faster than the allowance"
            }
        )
    };
    let closed = closed.expect("the socket closed without saying why");
    assert_eq!(closed.code, CloseCode::Policy, "{closed:?}");
    // Counted in proportion to what was read, however fast that was: past
    // the burst, and no further than the allowance admits.
    let read = logged(&gw, "over its frame allowance", "frames")
        .await
        .expect("the gateway says how much it read before it closes a seat");
    assert!(
        read > BURST,
        "closed after {read} frames, inside the burst of {BURST}"
    );
    assert!(
        read <= ceiling,
        "closed after {read} frames in {elapsed:?}; the allowance admits {ceiling}"
    );

    // Deliberately withhold the reply that tungstenite queued on receipt.
    // Reading TCP directly cannot flush it: the server must expire its own
    // closing deadline instead of retaining a silent peer indefinitely.
    //
    // That deadline is one second from the gateway's close frame
    // (`close_over_allowance`), and this clock starts only once that frame
    // has been read here, so three seconds leave two for a loaded scheduler.
    let tokio_tungstenite::MaybeTlsStream::Plain(tcp) = ws.get_mut() else {
        panic!("the fixture uses numeric loopback without TLS");
    };
    let mut byte = [0];
    let ended = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        tokio::io::AsyncReadExt::read(tcp, &mut byte),
    )
    .await
    .expect("a peer withholding its close reply must expire")
    .expect("the drained flood must close cleanly");
    assert_eq!(ended, 0, "the gateway sent data after its close frame");
}

/// The gateway's bound on one seat frame, `MAX_SEAT_FRAME`: 64 KiB.
const MAX_SEAT_FRAME: usize = 64 * 1024;

/// A frame up to the bound is forwarded and the socket goes on; one byte
/// more and the socket refuses it as it arrives, before it is read whole,
/// and closes. Under the old check the gateway read any message the
/// transport allowed (64 MiB) and only then dropped it.
#[tokio::test]
async fn a_seat_frame_past_the_bound_closes_the_socket_and_one_at_it_does_not() {
    let gw = spawn_gateway("seat-bound");
    let port = gw.port;
    let _agent = common::attach_agent(&gw).await;
    let token = login(port, "bounded", "bounded");
    let (status, body) = http(
        port,
        "POST",
        "/decks",
        Some(&token),
        "{\"name\":\"d\",\"cards\":[\"40 Forest\",\"20 Swamp\"]}",
    );
    assert_eq!(status, 200, "create deck: {body}");
    let create = format!(
        "{{\"deck_id\":\"{}\",\"mode\":\"ai\"}}",
        json_field(&body, "deck_id")
    );
    let (status, body) = http(port, "POST", "/lobby/games", Some(&token), &create);
    assert_eq!(status, 200, "a game against the house: {body}");
    let mut ws = common::dial_seat(
        port,
        json_field(&body, "game_id"),
        json_field(&body, "seat_token"),
    )
    .await;

    // At the bound: bytes no engine can read, so it is dropped there, and
    // the socket still answers a probe after it.
    ws.send(Message::Binary(vec![0xff; MAX_SEAT_FRAME].into()))
        .await
        .expect("a frame at the bound is sent");
    let probe = Envelope {
        msg: Some(v1::envelope::Msg::ClockProbe(v1::ClockProbe {
            client_time_ms: 4242,
            server_time_ms: 0,
        })),
    };
    common::send(&mut ws, &probe).await;
    let answered = tokio::time::timeout(common::WAIT_BUDGET, async {
        while let Some(msg) = common::next_msg(&mut ws).await {
            if let v1::envelope::Msg::ClockProbe(back) = msg {
                return back.client_time_ms == 4242;
            }
        }
        false
    })
    .await
    .unwrap_or(false);
    assert!(answered, "a frame at the bound cost the socket its life");

    // Past it: the socket is gone, whatever it says on the way.
    let _ = ws
        .send(Message::Binary(vec![0xff; MAX_SEAT_FRAME + 1].into()))
        .await;
    let ended = tokio::time::timeout(common::WAIT_BUDGET, async {
        loop {
            match ws.next().await {
                None | Some(Err(_) | Ok(Message::Close(_))) => return true,
                Some(Ok(_)) => {}
            }
        }
    })
    .await
    .unwrap_or(false);
    assert!(ended, "a frame past the bound was taken");
}
