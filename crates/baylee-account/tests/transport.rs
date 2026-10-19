//! Real loopback HTTP checks for the credential transport; no live credentials or services.
#![forbid(unsafe_code)]
use baylee_account::{
    CentralHttp, Endpoint, Error,
    wire::{Request, Response},
};
use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
    time::Duration,
};

fn exchange(status: u16, headers: &str, body: &[u8]) -> Result<Response, Error> {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint =
        Endpoint::development(&format!("http://{}", listener.local_addr().unwrap())).unwrap();
    let reply = format!(
        "HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nConnection: close\r\n{headers}\r\n",
        body.len()
    );
    let body = body.to_vec();
    let server = thread::spawn(move || {
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
        let size: usize = header
            .lines()
            .find_map(|line| {
                line.to_lowercase()
                    .strip_prefix("content-length: ")
                    .map(str::to_owned)
            })
            .unwrap()
            .parse()
            .unwrap();
        let mut request = vec![0; size];
        stream.read_exact(&mut request).unwrap();
        assert!(header.starts_with("POST /v1/native HTTP/1.1"));
        assert_eq!(
            request,
            br#"{"operation":"discover","username":"synthetic"}"#
        );
        let _ = stream.write_all(reply.as_bytes());
        let _ = stream.write_all(&body);
    });
    let result = CentralHttp::new(endpoint).exchange(
        &Request::Discover {
            username: "synthetic".into(),
        },
        None,
    );
    server.join().unwrap();
    result
}

#[test]
fn redirects_never_forward_credentials_to_the_location() {
    let sink = TcpListener::bind("127.0.0.1:0").unwrap();
    sink.set_nonblocking(true).unwrap();
    let headers = format!("Location: http://{}/stolen\r\n", sink.local_addr().unwrap());
    assert!(exchange(307, &headers, b"").is_err());
    assert!(matches!(sink.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock));
}

#[test]
fn untrusted_bodies_types_sizes_and_error_diagnostics_are_rejected() {
    assert!(matches!(
        exchange(
            200,
            "Content-Type: application/json\r\n",
            br#"{"result":"done"}"#
        ),
        Ok(Response::Done)
    ));
    assert!(exchange(200, "Content-Type: text/html\r\n", br#"{"result":"done"}"#).is_err());
    assert!(
        exchange(
            200,
            "Content-Type: application/json\r\n",
            &vec![b' '; 1_048_577]
        )
        .is_err()
    );
    assert!(matches!(
        exchange(
            500,
            "Content-Type: application/json\r\n",
            b"REMOTE_PRIVATE_CANARY"
        ),
        Err(Error::Unavailable)
    ));
    assert!(matches!(
        exchange(409, "", b"REMOTE_PRIVATE_CANARY"),
        Err(Error::Conflict)
    ));
}
