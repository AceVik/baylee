use super::super::{
    cli::{Cli, Command},
    control,
};
use super::*;
use clap::Parser;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::thread;
use std::time::Duration;

fn server(status: &str, headers: &str, body: Vec<u8>) -> (u16, thread::JoinHandle<String>) {
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
    let port = listener.local_addr().unwrap().port();
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n{headers}\r\n",
        body.len()
    );
    let thread = thread::spawn(move || {
        let (socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        socket
            .set_write_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut reader = BufReader::new(socket);
        let mut request = String::new();
        let mut length = None;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            assert!(!line.is_empty(), "incomplete HTTP request");
            request.push_str(&line);
            let lower = line.to_ascii_lowercase();
            assert!(
                !lower.starts_with("transfer-encoding:"),
                "dev-control rejects chunked bodies"
            );
            if let Some(value) = lower.strip_prefix("content-length:") {
                assert!(length.is_none(), "duplicate Content-Length");
                length = Some(value.trim().parse::<usize>().unwrap());
            }
            if line == "\r\n" {
                break;
            }
            assert!(request.len() < 8192);
        }
        if request.starts_with("POST ") {
            assert!(length.is_some(), "POST must have a fixed Content-Length");
        }
        let length = length.unwrap_or(0);
        assert!(length <= 1024);
        let mut payload = vec![0; length];
        reader.read_exact(&mut payload).unwrap();
        request.push_str(std::str::from_utf8(&payload).unwrap());
        let mut socket = reader.into_inner();
        socket.write_all(response.as_bytes()).unwrap();
        // An oversized-body test intentionally closes the client before EOF.
        let _ = socket.write_all(&body);
        request
    });
    (port, thread)
}

#[test]
fn status_and_cancel_use_loopback_and_expected_verbs() {
    let (port, peer) = server("200 OK", "", br#"{"ok":true,"phase":"idle"}"#.to_vec());
    assert_eq!(
        control::request(port, "status", None).unwrap()["phase"],
        "idle"
    );
    assert!(
        peer.join()
            .unwrap()
            .starts_with("GET /recording/status HTTP/1.1\r\n")
    );
    for body in [json!({"cancel":true}), json!({"cancel":false}), json!({})] {
        let (port, peer) = server("200 OK", "", br#"{"ok":true}"#.to_vec());
        control::request(port, "stop", Some(&body)).unwrap();
        let request = peer.join().unwrap();
        assert!(request.starts_with("POST /recording/stop HTTP/1.1\r\n"));
        assert_eq!(request.split_once("\r\n\r\n").unwrap().1, body.to_string());
    }
}

#[test]
fn redirects_error_bodies_and_oversized_responses_are_rejected() {
    for (status, headers, body) in [
        (
            "302 Found",
            "Location: http://192.0.2.1/private\r\n",
            b"{}".to_vec(),
        ),
        ("200 OK", "", br#"{"error":"busy"}"#.to_vec()),
        ("500 Failure", "", b"{}".to_vec()),
        ("200 OK", "", vec![b' '; 1024 * 1024 + 1]),
    ] {
        let (port, peer) = server(status, headers, body);
        assert!(control::request(port, "status", None).is_err());
        peer.join().unwrap();
    }
    assert!(control::request(0, "status", None).is_err());
    assert!(control::request(1, "../other", None).is_err());
    let oversized = json!({"unknown":"x".repeat(1024)});
    let error = control::request(1, "stop", Some(&oversized)).unwrap_err();
    assert!(error.to_string().contains("body exceeds 1 KiB"));
}

#[test]
fn cli_defaults_options_and_boundaries_match_capture_contract() {
    let Cli {
        command:
            Command::Record {
                port,
                duration_ms,
                fps,
                preview: options,
            },
    } = Cli::try_parse_from(["recording", "record"]).unwrap()
    else {
        panic!("record variant")
    };
    assert_eq!((port, duration_ms, fps), (28770, 3000, 15));
    assert_eq!((options.width, options.sheet_frames), (640, 12));
    assert_eq!(options.format, Format::Webp);
    for args in [
        vec!["status", "--port", "0"],
        vec!["status", "--port", "65536"],
        vec!["record", "--duration-ms", "30001"],
        vec!["record", "--fps", "61"],
        vec!["encode", "/tmp", "--width", "15"],
        vec!["encode", "/tmp", "--sheet-frames", "49"],
        vec!["encode", "/tmp", "--cadence", "display"],
    ] {
        assert!(Cli::try_parse_from(std::iter::once("recording").chain(args)).is_err());
    }
    assert!(Cli::try_parse_from(["recording", "stop", "--cancel"]).is_ok());
    assert!(
        Cli::try_parse_from([
            "recording",
            "encode",
            "/tmp",
            "--allow-incomplete",
            "--diff",
            "--format",
            "both",
            "--cadence",
            "virtual"
        ])
        .is_ok()
    );
    assert!(control::record(1, 30_000, 60, &preview()).is_err());
}

#[test]
fn record_start_encodes_completed_run_and_sends_exact_capture_options() {
    let dir = tempfile::tempdir().unwrap();
    fixture(dir.path());
    let body = json!({"ok":true,"recording":{"directory":dir.path(),"phase":"complete"}});
    let (port, peer) = server("200 OK", "", body.to_string().into_bytes());
    let result = control::record(port, 500, 10, &preview()).unwrap();
    assert_eq!(result["saved_frames"], 3);
    let request = peer.join().unwrap();
    assert!(request.starts_with("POST /recording/start HTTP/1.1\r\n"));
    let payload: Value = serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap();
    assert_eq!(payload, json!({"duration_ms":500,"fps":10}));
}
