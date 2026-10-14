use super::{WsEvent, WsMessage, WsReceiver, WsSender, socket_config, ws_connect};
use std::io::ErrorKind;
use std::net::{TcpListener, TcpStream};
use std::time::{Duration, Instant};
use tungstenite::protocol::frame::Frame;
use tungstenite::protocol::frame::coding::{Data, OpCode};
use tungstenite::{Message, WebSocket};

const WAIT: Duration = Duration::from_secs(5);

// Accept, handshake, peer I/O and event waits are all bounded. No gateway or GPU.
fn loopback(options: ewebsock::Options) -> (WsSender, WsReceiver, WebSocket<TcpStream>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    listener.set_nonblocking(true).expect("nonblocking accept");
    let url = format!("ws://{}/transport-test", listener.local_addr().unwrap());
    let (sender, receiver) = ws_connect(url, options).expect("start native client");
    let deadline = Instant::now() + WAIT;
    let stream = loop {
        match listener.accept() {
            Ok((stream, _)) => break stream,
            Err(err) if err.kind() == ErrorKind::WouldBlock => {
                assert!(Instant::now() < deadline, "client did not connect");
                std::thread::sleep(Duration::from_millis(1));
            }
            Err(err) => panic!("accept: {err}"),
        }
    };
    // Darwin inherits the listener's nonblocking mode. The synchronous peer
    // needs blocking I/O; the timeouts below still bound handshake and frames.
    stream.set_nonblocking(false).expect("blocking peer socket");
    stream.set_read_timeout(Some(WAIT)).unwrap();
    stream.set_write_timeout(Some(WAIT)).unwrap();
    stream.set_nodelay(true).unwrap();
    let peer = tungstenite::accept(stream).expect("websocket handshake");
    assert!(matches!(next_event(&receiver), WsEvent::Opened));
    (sender, receiver, peer)
}

fn next_event(receiver: &WsReceiver) -> WsEvent {
    let deadline = Instant::now() + WAIT;
    loop {
        if let Some(event) = receiver.try_recv() {
            return event;
        }
        assert!(Instant::now() < deadline, "native socket produced no event");
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn socket_config_keeps_the_legacy_limits_and_read_buffer() {
    for (limit, expected) in [
        (
            ewebsock::Options::default().max_incoming_frame_size,
            Some(64 << 20),
        ),
        (0, Some(0)),
        (17, Some(17)),
        (usize::MAX, None),
    ] {
        let config = socket_config(&ewebsock::Options {
            max_incoming_frame_size: limit,
            ..Default::default()
        });
        assert_eq!(config.max_frame_size, expected);
        assert_eq!(config.read_buffer_size, 4096);
        // A frame limit must not become a limit on the reassembled message.
        assert_eq!(config.max_message_size, Some(64 << 20));
        assert_eq!(config.write_buffer_size, 128 << 10);
        assert_eq!(config.max_write_buffer_size, usize::MAX);
        assert!(!config.accept_unmasked_frames);
    }
}

#[test]
fn socket_round_trips_text_binary_ping_and_pong() {
    let (mut sender, receiver, mut peer) = loopback(ewebsock::Options::default());
    // Unknown remains unsendable; the next frame must be the first text below.
    sender.send(WsMessage::Unknown("not a frame".to_owned()));
    for text in ["", "Grüße 🦀\0日本語"] {
        sender.send(WsMessage::Text(text.to_owned()));
        assert_eq!(peer.read().unwrap(), Message::text(text));
        peer.send(Message::text(text)).unwrap();
        let event = next_event(&receiver);
        assert!(matches!(event, WsEvent::Message(WsMessage::Text(ref got)) if got == text));
    }
    // All byte values, invalid UTF-8 included, crossing two 4 KiB read chunks.
    for bytes in [Vec::new(), (0..=255_u8).cycle().take(8193).collect()] {
        sender.send(WsMessage::Binary(bytes.clone()));
        assert_eq!(peer.read().unwrap(), Message::binary(bytes.clone()));
        peer.send(Message::binary(bytes.clone())).unwrap();
        let event = next_event(&receiver);
        assert!(matches!(event, WsEvent::Message(WsMessage::Binary(ref got)) if *got == bytes));
    }

    let ping = vec![0, 255, 128, 42];
    sender.send(WsMessage::Ping(ping.clone()));
    assert_eq!(peer.read().unwrap(), Message::Ping(ping.clone().into()));
    peer.flush().unwrap(); // Flush the peer's automatic pong.
    let event = next_event(&receiver);
    assert!(matches!(event, WsEvent::Message(WsMessage::Pong(ref got)) if *got == ping));

    let pong = vec![254, 0, 127];
    sender.send(WsMessage::Pong(pong.clone()));
    assert_eq!(peer.read().unwrap(), Message::Pong(pong.into()));
    peer.send(Message::Ping(ping.clone().into())).unwrap();
    let event = next_event(&receiver);
    assert!(matches!(event, WsEvent::Message(WsMessage::Ping(ref got)) if *got == ping));
    // Receiving a ping still sends its automatic pong with the same bytes.
    assert_eq!(peer.read().unwrap(), Message::Pong(ping.into()));

    peer.close(None).unwrap();
    assert!(matches!(next_event(&receiver), WsEvent::Closed));
}

fn send_fragment(peer: &mut WebSocket<TcpStream>, bytes: &[u8], kind: Data, last: bool) {
    peer.send(Message::Frame(Frame::message(
        bytes.to_vec(),
        OpCode::Data(kind),
        last,
    )))
    .unwrap();
}

#[test]
fn socket_reassembles_fragments_and_answers_interleaved_ping() {
    let (_sender, receiver, mut peer) = loopback(ewebsock::Options {
        max_incoming_frame_size: 8,
        ..Default::default()
    });
    let bytes: Vec<u8> = (240..=255).collect();
    send_fragment(&mut peer, &bytes[..8], Data::Binary, false);
    peer.send(Message::Ping(vec![0, 255].into())).unwrap();
    let event = next_event(&receiver);
    assert!(matches!(event, WsEvent::Message(WsMessage::Ping(ref got)) if got == &[0, 255]));
    assert_eq!(peer.read().unwrap(), Message::Pong(vec![0, 255].into()));
    send_fragment(&mut peer, &bytes[8..], Data::Continue, true);
    let event = next_event(&receiver);
    assert!(matches!(event, WsEvent::Message(WsMessage::Binary(ref got)) if *got == bytes));

    // A UTF-8 code point may straddle frames; validate the complete message.
    let text = "Grüße";
    send_fragment(&mut peer, &text.as_bytes()[..3], Data::Text, false);
    send_fragment(&mut peer, &text.as_bytes()[3..], Data::Continue, true);
    let event = next_event(&receiver);
    assert!(matches!(event, WsEvent::Message(WsMessage::Text(ref got)) if got == text));
}

#[test]
fn socket_enforces_the_frame_limit_including_zero() {
    for (limit, length, allowed) in [(0, 0, true), (0, 1, false), (8, 8, true), (8, 9, false)] {
        let (_sender, receiver, mut peer) = loopback(ewebsock::Options {
            max_incoming_frame_size: limit,
            ..Default::default()
        });
        let bytes = vec![255; length];
        peer.send(Message::binary(bytes.clone())).unwrap();
        let event = next_event(&receiver);
        if allowed {
            assert!(matches!(event, WsEvent::Message(WsMessage::Binary(ref got)) if *got == bytes));
        } else {
            assert!(matches!(event, WsEvent::Error(ref error) if error.starts_with("read:")));
        }
    }
}

#[test]
fn socket_sends_after_idle_reads_for_all_timeout_options() {
    for read_timeout in [None, Some(Duration::ZERO), Some(Duration::from_millis(25))] {
        let (mut sender, _receiver, mut peer) = loopback(ewebsock::Options {
            read_timeout,
            ..Default::default()
        });
        // Let the worker enter its read while the peer is silent. None/zero
        // must still fall back to a finite wait so later sends get drained.
        std::thread::sleep(Duration::from_millis(50));
        sender.send(WsMessage::Binary(vec![0, 128, 255]));
        assert_eq!(peer.read().unwrap(), Message::binary(vec![0, 128, 255]));
        drop(sender);
        assert!(matches!(peer.read().unwrap(), Message::Close(None)));
    }
}
