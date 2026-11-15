use super::*;
mod location;
use std::{
    io::Write,
    net::{TcpListener, TcpStream},
    thread,
    time::{Duration, Instant},
};

fn reply(status: u16, headers: &str, body: &[u8]) -> Result<Response, Error> {
    reply_to(&Request::Runs { before: None }, status, headers, body)
}

fn reply_to(asked: &Request, status: u16, headers: &str, body: &[u8]) -> Result<Response, Error> {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let token = central_api::Token::generate().unwrap();
    let digest = token.digest();
    let client = TowerHttp::new(
        &Endpoint::development("http://127.0.0.1:1").unwrap(),
        *uuid::Uuid::now_v7().as_bytes(),
        Grant {
            origin: format!("http://{address}"),
            token,
            expires_at: 300,
        },
    )
    .unwrap();
    let response = format!(
        "HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nConnection: close\r\n{headers}\r\n",
        body.len()
    );
    let body = body.to_vec();
    let expected = asked.clone();
    let server = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(error)
                    if error.kind() == std::io::ErrorKind::WouldBlock
                        && Instant::now() < deadline =>
                {
                    thread::sleep(Duration::from_millis(5));
                }
                _ => panic!("synthetic listener did not receive the bounded request"),
            }
        };
        // macOS may inherit the listener's nonblocking mode on accepted sockets.
        // The listener polls admission; each owned connection uses bounded blocking I/O.
        stream.set_nonblocking(false).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut header = Vec::new();
        while !header.ends_with(b"\r\n\r\n") && header.len() < 8192 {
            let mut byte = [0];
            stream.read_exact(&mut byte).unwrap();
            header.push(byte[0]);
        }
        let header = String::from_utf8(header).unwrap();
        assert!(header.starts_with("POST /v2/player HTTP/1.1"));
        let fields: Vec<_> = header
            .lines()
            .filter_map(|line| line.split_once(": "))
            .collect();
        let value = |name: &str| {
            fields
                .iter()
                .find(|(key, _)| key.eq_ignore_ascii_case(name))
                .map(|(_, value)| *value)
                .unwrap()
        };
        assert_eq!(value(api::VERSION_HEADER), api::VERSION);
        let credential = central_api::Token::from_bearer(value("authorization")).unwrap();
        assert_eq!(credential.digest(), digest);
        let size: usize = value("content-length").parse().unwrap();
        assert!(size <= api::MAX_REQUEST_BYTES);
        let mut request = vec![0; size];
        stream.read_exact(&mut request).unwrap();
        assert_eq!(api::decode(&request).unwrap(), expected);
        let _ = stream.write_all(response.as_bytes());
        let _ = stream.write_all(&body);
    });
    let result = client.exchange(asked);
    server.join().unwrap();
    assert!(TcpStream::connect_timeout(&address, Duration::from_secs(1)).is_err());
    result
}

#[test]
fn tower_transport_binds_scope_and_refuses_redirects_and_untrusted_responses() {
    let valid = "Content-Type: application/json\r\nBaylee-Tower-Version: 2\r\n";
    let runs = br#"{"result":"runs","runs":[]}"#;
    assert!(matches!(reply(200, valid, runs), Ok(Response::Runs { runs }) if runs.is_empty()));
    let sink = TcpListener::bind("127.0.0.1:0").unwrap();
    sink.set_nonblocking(true).unwrap();
    assert!(
        reply(
            307,
            &format!("Location: http://{}/stolen\r\n", sink.local_addr().unwrap()),
            b""
        )
        .is_err()
    );
    assert!(matches!(sink.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock));
    drop(sink);
    assert!(reply(200, "Content-Type: application/json\r\n", runs).is_err());
    assert!(
        reply(
            200,
            "Content-Type: text/html\r\nBaylee-Tower-Version: 2\r\n",
            runs
        )
        .is_err()
    );
    assert!(reply(200, valid, br#"{"result":"withdrawn"}"#).is_err());
    assert!(
        reply(
            200,
            valid,
            br#"{"result":"runs","runs":[],"token":"untrusted"}"#
        )
        .is_err()
    );
    assert!(reply(200, valid, &vec![b' '; api::MAX_RESPONSE_BYTES + 1]).is_err());
    assert!(matches!(
        reply(503, valid, b"PRIVATE_REMOTE_DIAGNOSTICS"),
        Err(Error::Unavailable)
    ));
    assert!(matches!(reply(401, valid, b""), Err(Error::Rejected)));
    assert!(matches!(reply(409, valid, b""), Err(Error::Conflict)));
    assert!(matches!(reply(429, valid, b""), Err(Error::Busy)));
}

#[test]
fn production_and_development_cannot_exchange_credential_destinations() {
    for (central, origin) in [
        (Endpoint::live(), "http://127.0.0.1:29871"),
        (Endpoint::live(), central_api::LIVE_ORIGIN),
        (
            Endpoint::development("http://127.0.0.1:29871").unwrap(),
            "https://tower.example.invalid",
        ),
        (
            Endpoint::development("http://127.0.0.1:29871").unwrap(),
            "http://127.0.0.1:29871",
        ),
    ] {
        assert!(
            TowerHttp::new(
                &central,
                *uuid::Uuid::now_v7().as_bytes(),
                Grant {
                    origin: origin.into(),
                    token: central_api::Token::generate().unwrap(),
                    expires_at: 300
                }
            )
            .is_err()
        );
    }
}
