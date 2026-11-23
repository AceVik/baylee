use super::*;
use crate::{
    host::DuelHost as _,
    net::{NetworkHost, SeatTicket},
};
use baylee_core::ids::PlayerId;
use baylee_protocol::v1::{self, Envelope, envelope::Msg};
use std::{net::TcpListener, sync::mpsc, thread};

fn status(generation: u64) -> SeatStatus {
    SeatStatus {
        consent: ConsentState::Ready,
        fence: Fence {
            match_id: [7; 16],
            generation,
        },
        host_id: [u8::try_from(generation).unwrap(); 16],
        origin: format!("https://host-{generation}.invalid"),
        game: Some([9; 16]),
        phase: Phase::Playing,
        head: Head {
            position: 0,
            chain: [1; 32],
        },
        clock: Some(baylee_account::ranked::replica::clock::ClockView {
            decision_secs: [Some(90), None],
            reconnect_secs: [None, None],
            running: true,
        }),
    }
}

fn answer() -> Envelope {
    Envelope {
        msg: Some(Msg::SeatSetting(v1::SeatSettingMsg { setting: vec![3] })),
    }
}

fn projection(generation: u64, head: Head) -> Envelope {
    Envelope {
        msg: Some(Msg::RankedSeatProjection(v1::RankedSeatProjection {
            generation,
            position: head.position,
            chain: head.chain.to_vec(),
            envelope: answer().encode_to_vec(),
        })),
    }
}

#[test]
fn an_ambiguous_command_keeps_its_uuid_bytes_and_expected_position_across_handover() {
    let table = Table::default();
    let first = status(1);
    table.observe(first.clone());
    let mut old = table.attach(&first).unwrap();
    old.has_view = true;
    table.lock().shown = Some(first.head);
    old.enqueue(&answer());
    let now = Instant::now();
    let Some(SeatRequest::Submit {
        command_id,
        expected,
        body,
        ..
    }) = table.submission(now)
    else {
        panic!("missing submission")
    };
    table.answered(command_id, Err(Error::Unavailable), now);
    let mut next = status(2);
    next.head = Head {
        position: 1,
        chain: [2; 32],
    };
    table.observe(next.clone());
    let mut new = table.attach(&next).unwrap();
    assert!(old.decode(projection(1, first.head)).is_none());
    assert!(
        new.decode(answer()).is_none(),
        "no unwrapped mirrored gameplay"
    );
    assert!(new.decode(projection(1, next.head)).is_none());
    assert!(
        new.decode(projection(
            2,
            Head {
                position: 1,
                chain: [99; 32]
            }
        ))
        .is_none()
    );
    drop(old);
    assert!(
        new.current(),
        "old socket destructor cannot detach its replacement"
    );
    let Some(SeatRequest::Submit {
        fence,
        command_id: retry,
        expected: parent,
        body: bytes,
    }) = table.submission(now + Duration::from_secs(3))
    else {
        panic!("missing retry")
    };
    assert_eq!(
        (fence, retry, parent, bytes),
        (next.fence, command_id, expected, body)
    );
    table.answered(command_id, Ok(1), now);
    assert!(
        new.decode(projection(2, first.head)).is_none(),
        "fresh seat cannot reopen behind its receipt"
    );
    assert!(new.decode(projection(2, next.head)).is_some());
    new.has_view = true;
    table.lock().shown = Some(next.head);
    assert_eq!(new.hold(), None);
}

#[test]
fn disconnect_conflict_and_new_host_consent_never_resubmit_a_rebased_choice() {
    let table = Table::default();
    let mut first = status(1);
    table.observe(first.clone());
    let mut socket = table.attach(&first).unwrap();
    socket.has_view = true;
    table.lock().shown = Some(first.head);
    socket.enqueue(&answer());
    let now = Instant::now();
    let Some(SeatRequest::Submit { command_id, .. }) = table.submission(now) else {
        panic!("missing submission")
    };
    table.answered(command_id, Err(Error::Conflict), now);
    assert!(table.submission(now).is_none());
    assert!(table.needs_ticket());
    assert!(!socket.current());
    first.consent = ConsentState::Needed;
    table.observe(first.clone());
    assert!(!table.needs_ticket());
    assert!(table.submission(now).is_none());
}

#[test]
fn the_ranked_relay_uses_only_a_fixed_trusted_socket_path() {
    assert_eq!(
        socket_url("https://central.invalid:444").unwrap(),
        "wss://central.invalid:444/v1/ranked/socket"
    );
    for invalid in [
        "http://example.org",
        "https://user@central.invalid",
        "https://central.invalid?ticket=x",
        "https://central.invalid/path",
        "ws://127.0.0.1",
    ] {
        assert!(socket_url(invalid).is_err(), "{invalid}");
    }
}

#[test]
fn protected_clocks_pause_without_a_standin_and_only_certified_presence_renews() {
    let table = Table::default();
    let mut status = status(1);
    table.observe(status.clone());
    let mut socket = table.attach(&status).unwrap();
    let now = Instant::now();
    socket.connection(true);
    assert!(table.take_presence(now).is_none());
    socket.has_view = true;
    socket.connection(true);
    assert_eq!(table.take_presence(now), Some((status.fence, true)));
    assert!(table.take_presence(now + Duration::from_secs(3)).is_none());
    assert_eq!(
        table.take_presence(now + Duration::from_secs(4)),
        Some((status.fence, true))
    );
    status.phase = Phase::Paused;
    let clock = status.clock.as_mut().unwrap();
    clock.running = false;
    clock.reconnect_secs = [None, Some(0)];
    table.observe(status);
    assert_eq!(socket.hold(), Some(Phrase::RankedVerifierHold));
    let me = PlayerId::new(0);
    let mut duel = crate::Duel::default();
    duel.sync_ranked_clocks(me, &socket.clocks(false).unwrap());
    assert!(duel.clock_paused());
    assert_eq!(duel.clock.shown(), Some(90));
    assert_eq!(duel.seat_clocks.shown(me), Some(90));
    assert_eq!(
        duel.lost_seats.ranked_banner(me),
        Some(baylee_client_core::lostbanner::Banner::RankedExpired)
    );
    socket.lost();
    assert_eq!(
        table
            .take_presence(now + Duration::from_secs(5))
            .map(|(_, connected)| connected),
        Some(false)
    );
}

#[test]
#[expect(
    clippy::result_large_err,
    reason = "tungstenite specifies the handshake callback's HTTP error response type"
)]
fn central_receives_the_single_use_ticket_in_the_first_binary_frame_in_both_modes() {
    use tungstenite::handshake::server::{Request, Response};
    for phase in [Phase::Single, Phase::Playing] {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let (send, receive) = mpsc::sync_channel(1);
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut socket =
                tungstenite::accept_hdr(stream, |request: &Request, response: Response| {
                    assert_eq!(request.uri().to_string(), "/v1/ranked/socket");
                    assert!(request.headers().get("authorization").is_none());
                    Ok(response)
                })
                .unwrap();
            let tungstenite::Message::Binary(first) = socket.read().unwrap() else {
                panic!("binary ticket expected")
            };
            send.send(first).unwrap();
        });
        let table = Table::default();
        let mut status = status(1);
        status.phase = phase;
        table.observe(status.clone());
        let secret = "ac".repeat(32);
        let mut host = NetworkHost::ranked(
            SeatTicket {
                gateway: origin,
                game_id: "public-test-game".into(),
                seat: PlayerId::new(1),
                seat_token: secret.clone(),
                watch: false,
            },
            table.attach(&status).unwrap(),
        )
        .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            host.poll();
            if let Ok(first) = receive.try_recv() {
                assert_eq!(first.as_ref(), secret.as_bytes());
                break;
            }
            assert!(Instant::now() < deadline, "relay handshake timed out");
            thread::sleep(Duration::from_millis(4));
        }
        server.join().unwrap();
        assert_eq!(host.seat_token(), None);
    }
}
