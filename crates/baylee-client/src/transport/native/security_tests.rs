//! Offline, bounded peers verify the shipped transport's trust boundaries.

use super::{fetch_blocking_limited, tls, ws_connect};
use ewebsock::WsEvent;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::time::{Duration, Instant};

const WAIT: Duration = Duration::from_secs(5);

fn empty_outgoing() -> super::sender::SocketQueue {
    let (receiver, publish) = super::WsReceiver::new();
    let (_sender, outgoing) = super::sender::channel(publish, receiver.cancellation());
    outgoing
}

#[test]
fn malformed_socket_url_diagnostics_do_not_contain_the_ticket() {
    let receive = empty_outgoing();
    let error = super::run_socket(
        "wss://example.invalid/\0?token=synthetic-private-seat-ticket",
        &ewebsock::Options::default(),
        &|_| std::ops::ControlFlow::Break(()),
        &receive,
    )
    .expect_err("malformed URL must fail before connecting");
    assert!(error.contains("URL"));
    assert!(!error.contains("synthetic-private-seat-ticket"));
    assert!(!error.contains("example.invalid"));
}

#[test]
fn socket_refusal_diagnostics_exclude_echoed_credentials() {
    const TICKET: &str = "synthetic-private-seat-ticket";
    let source = listener();
    let address = source.local_addr().unwrap();
    let peer = std::thread::spawn(move || {
        let mut stream = accept(&source);
        read_request(&mut stream);
        write!(
            stream,
            "HTTP/1.1 401 Unauthorized\r\nX-Echo-Ticket: {TICKET}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{TICKET}",
            TICKET.len(),
        )
        .unwrap();
    });
    let receive = empty_outgoing();
    let result = super::run_socket(
        &format!("ws://{address}/socket?token={TICKET}"),
        &ewebsock::Options::default(),
        &|_| std::ops::ControlFlow::Break(()),
        &receive,
    );
    peer.join().expect("refusal peer exited");
    let error = result.expect_err("HTTP refusal must fail the socket upgrade");
    assert_eq!(error, "Connect: HTTP error: 401 Unauthorized");
    assert!(!error.contains(TICKET));
    assert!(!error.contains("X-Echo-Ticket"));
}

#[test]
fn cleartext_transport_is_confined_to_numeric_loopback() {
    for (clear, secure) in [("http", "https"), ("ws", "wss")] {
        for host in ["127.0.0.1", "127.100.2.3", "[::1]"] {
            let uri = format!("{clear}://{host}:9000/path").parse().unwrap();
            assert!(super::check_address(&uri, clear, secure).is_ok());
        }
        for host in [
            "localhost",
            "example.test",
            "192.168.1.1",
            "0.0.0.0",
            "[::]",
            "[::ffff:127.0.0.1]",
        ] {
            let uri = format!("{clear}://{host}:9000/path").parse().unwrap();
            assert!(super::check_address(&uri, clear, secure).is_err());
        }
        let uri = format!("{secure}://example.test/path").parse().unwrap();
        assert!(super::check_address(&uri, clear, secure).is_ok());
        let userinfo = format!("{secure}://user:secret@example.test/path")
            .parse()
            .unwrap();
        assert!(super::check_address(&userinfo, clear, secure).is_err());
    }
}

fn listener() -> TcpListener {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    listener
}

fn accept(listener: &TcpListener) -> TcpStream {
    let deadline = Instant::now() + WAIT;
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                stream.set_nonblocking(false).unwrap();
                stream.set_read_timeout(Some(WAIT)).unwrap();
                stream.set_write_timeout(Some(WAIT)).unwrap();
                return stream;
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                assert!(Instant::now() < deadline, "fixture client did not connect");
                std::thread::sleep(Duration::from_millis(1));
            }
            Err(error) => panic!("accept: {error}"),
        }
    }
}

fn untrusted_tls_peer() -> (std::net::SocketAddr, std::thread::JoinHandle<bool>) {
    let listener = listener();
    let address = listener.local_addr().unwrap();
    let peer = std::thread::spawn(move || {
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let config = rustls::ServerConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .unwrap()
            .with_no_client_auth()
            .with_single_cert(
                vec![rustls::pki_types::CertificateDer::from(
                    include_bytes!("test-data/localhost.der").to_vec(),
                )],
                rustls::pki_types::PrivatePkcs8KeyDer::from(
                    include_bytes!("test-data/localhost-key.der").to_vec(),
                )
                .into(),
            )
            .unwrap();
        let mut connection = rustls::ServerConnection::new(Arc::new(config)).unwrap();
        let mut stream = accept(&listener);
        while connection.is_handshaking() {
            if connection.complete_io(&mut stream).is_err() {
                return true;
            }
        }
        false
    });
    (address, peer)
}

#[test]
fn every_profile_rejects_an_untrusted_https_gateway_before_credentials() {
    assert!(!tls(false).disable_verification());
    assert!(!tls(true).disable_verification());
    let (address, peer) = untrusted_tls_peer();
    let mut request = ehttp::Request::post(
        format!("https://{address}/auth/login"),
        b"synthetic-test-password".to_vec(),
    );
    request.timeout = Some(WAIT);
    let response = fetch_blocking_limited(&request, 1024);
    let peer_rejected = peer.join().expect("TLS peer exited");
    assert!(response.is_err(), "untrusted TLS must reject in debug too");
    assert!(peer_rejected, "TLS handshake must fail before HTTP bytes");
}

#[test]
fn every_profile_rejects_an_untrusted_wss_gateway_before_upgrade() {
    let (address, peer) = untrusted_tls_peer();
    let (_sender, receiver) = ws_connect(
        format!("wss://{address}/socket?token=synthetic-test-ticket"),
        ewebsock::Options::default(),
    )
    .unwrap();
    let deadline = Instant::now() + WAIT;
    let event = loop {
        if let Some(event) = receiver.try_recv() {
            break event;
        }
        assert!(Instant::now() < deadline, "TLS refusal event missing");
        std::thread::sleep(Duration::from_millis(1));
    };
    let peer_rejected = peer.join().expect("WSS peer exited");
    assert!(matches!(event, WsEvent::Error(_)));
    assert!(
        peer_rejected,
        "TLS handshake must fail before upgrade bytes"
    );
}

fn read_request(stream: &mut TcpStream) {
    let mut header = Vec::new();
    while !header.ends_with(b"\r\n\r\n") {
        let mut byte = [0];
        stream.read_exact(&mut byte).unwrap();
        header.push(byte[0]);
        assert!(header.len() < 16_384);
    }
    let text = String::from_utf8(header).unwrap();
    let length = text
        .lines()
        .filter_map(|line| line.split_once(':'))
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .map_or(0, |(_, length)| length.trim().parse::<usize>().unwrap());
    assert!(length < 1024);
    stream.read_exact(&mut vec![0; length]).unwrap();
}

fn stalled_peer(
    tls: bool,
    http_body: bool,
    trickle: bool,
) -> (
    std::net::SocketAddr,
    std::sync::mpsc::Sender<()>,
    std::thread::JoinHandle<()>,
) {
    let listener = listener();
    let address = listener.local_addr().unwrap();
    let (release, wait) = std::sync::mpsc::channel();
    let peer = std::thread::spawn(move || {
        let mut stream = accept(&listener);
        if !tls {
            read_request(&mut stream);
        }
        if trickle && tls {
            // A TLS handshake record whose remaining payload never finishes.
            stream.write_all(&[22, 3, 3, 0x40, 0]).unwrap();
        } else if http_body {
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Length: 1000000\r\nConnection: close\r\n\r\n",
                )
                .unwrap();
        }
        let deadline = Instant::now() + WAIT;
        let header = b"HTTP/1.1 101 Switching Protocols\r\n";
        let mut at = 0;
        while Instant::now() < deadline {
            match wait.try_recv() {
                Ok(()) | Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
            }
            if trickle {
                let byte = if tls || http_body {
                    0
                } else {
                    header[at % header.len()]
                };
                if stream.write_all(&[byte]).is_err() {
                    break;
                }
                at += 1;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    });
    (address, release, peer)
}

#[test]
fn socket_handshake_has_an_absolute_budget_for_silent_and_trickling_peers() {
    for secure in [false, true] {
        for trickle in [false, true] {
            let (address, release, peer) = stalled_peer(secure, false, trickle);
            let scheme = if secure { "wss" } else { "ws" };
            let receive = empty_outgoing();
            let started = Instant::now();
            let answer = super::run_socket_with_budget(
                &format!("{scheme}://{address}/socket"),
                &ewebsock::Options::default(),
                &|_| std::ops::ControlFlow::Break(()),
                &receive,
                Duration::from_millis(100),
            );
            let elapsed = started.elapsed();
            let _ = release.send(());
            peer.join().expect("stalled socket peer exited");
            assert!(answer.is_err());
            assert!(
                elapsed < Duration::from_secs(2),
                "handshake waited {elapsed:?}"
            );
        }
    }
}

#[test]
fn http_request_budget_bounds_silent_and_trickling_response_bodies() {
    for trickle in [false, true] {
        let (address, release, peer) = stalled_peer(false, true, trickle);
        let request = ehttp::Request::get(format!("http://{address}/slow"))
            .with_timeout(Some(Duration::from_millis(100)));
        let started = Instant::now();
        let answer = fetch_blocking_limited(&request, 2 * 1024 * 1024);
        let elapsed = started.elapsed();
        let _ = release.send(());
        peer.join().expect("stalled HTTP peer exited");
        assert!(answer.is_err());
        assert!(
            elapsed < Duration::from_secs(2),
            "HTTP body waited {elapsed:?}"
        );
    }
}

#[test]
fn gateway_redirects_never_forward_bearer_headers_or_password_bodies() {
    for status in [301, 302, 303, 307, 308] {
        for password_body in [false, true] {
            let source = listener();
            let target = listener();
            let source_address = source.local_addr().unwrap();
            let target_address = target.local_addr().unwrap();
            let peer = std::thread::spawn(move || {
                let mut stream = accept(&source);
                read_request(&mut stream);
                write!(
                    stream,
                    "HTTP/1.1 {status} Redirect\r\nConnection: close\r\nLocation: http://{target_address}/capture\r\nContent-Length: 0\r\n\r\n"
                )
                .unwrap();
            });
            let url = format!("http://{source_address}/auth");
            let mut request = if password_body {
                ehttp::Request::post(url, b"synthetic-test-password".to_vec())
            } else {
                let mut request = ehttp::Request::get(url);
                request
                    .headers
                    .insert("Authorization", "Bearer synthetic-test-token");
                request
            };
            request.timeout = Some(WAIT);
            let response = fetch_blocking_limited(&request, 1024);
            peer.join().expect("redirect peer exited");
            let response = response.unwrap();
            assert_eq!(response.status, status);
            assert!(!response.ok);
            assert!(matches!(
                target.accept(),
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock
            ));
        }
    }
}
