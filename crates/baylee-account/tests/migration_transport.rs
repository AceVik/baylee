//! Only an explicit, schema-checked migration prerequisite can request a fresh source proof.
#![forbid(unsafe_code)]
use baylee_account::{
    Error, Token,
    migration::{LegacyHttp, LinkState},
};
use std::{
    io::{Read as _, Write as _},
    net::TcpListener,
    time::Duration,
};

fn link(status: u16, content_type: &str, body: &[u8]) -> Result<LinkState, Error> {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let reply = format!(
        "HTTP/1.1 {status} Test\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let body = body.to_vec();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
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
        assert!(header.starts_with("POST /auth/central/link HTTP/1.1"));
        let size: usize = header
            .lines()
            .find_map(|line| {
                line.to_ascii_lowercase()
                    .strip_prefix("content-length: ")
                    .map(str::to_owned)
            })
            .unwrap()
            .parse()
            .unwrap();
        assert!(size < 8192);
        stream.read_exact(&mut vec![0; size]).unwrap();
        let _ = stream.write_all(reply.as_bytes());
        let _ = stream.write_all(&body);
    });
    let result = LegacyHttp::development(&origin)
        .unwrap()
        .link_state(Token::generate().unwrap(), [0; 32]);
    server.join().unwrap();
    result
}

#[test]
fn only_closed_428_true_is_a_fresh_proof_prerequisite() {
    assert!(matches!(
        link(428, "application/json", br#"{"fresh_proof_required":true}"#),
        Ok(LinkState::FreshProofRequired)
    ));
    assert!(matches!(
        link(200, "application/json", b"{}"),
        Ok(LinkState::Complete)
    ));
    for (status, content_type, body) in [
        (
            428,
            "application/json",
            br#"{"fresh_proof_required":false}"#.as_slice(),
        ),
        (
            428,
            "application/json",
            br#"{"fresh_proof_required":true,"location":"https://other.invalid"}"#.as_slice(),
        ),
        (
            428,
            "text/html",
            br#"{"fresh_proof_required":true}"#.as_slice(),
        ),
        (
            401,
            "application/json",
            br#"{"fresh_proof_required":true}"#.as_slice(),
        ),
        (
            409,
            "application/json",
            br#"{"fresh_proof_required":true}"#.as_slice(),
        ),
        (
            503,
            "application/json",
            br#"{"fresh_proof_required":true}"#.as_slice(),
        ),
        (
            200,
            "application/json",
            br#"{"fresh_proof_required":true}"#.as_slice(),
        ),
    ] {
        assert!(link(status, content_type, body).is_err());
    }
}
