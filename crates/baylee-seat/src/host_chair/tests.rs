use super::*;
use clap::Parser as _;

fn input() -> serde_json::Value {
    serde_json::json!({"game_id":"room","seat":0,"seat_token":"seat-secret","session_token":"session-secret"})
}

#[test]
fn handoff_preserves_credentials_without_debugging_them() {
    let (session, chair) = parse(&input().to_string(), "room", Some(0)).unwrap();
    assert_eq!(chair.game_id, "room");
    assert_eq!(chair.seat, 0);
    assert_eq!(chair.seat_token, "seat-secret");
    assert!(!format!("{session:?} {chair:?}").contains("secret"));
}

/// A chair the account joined is handed over as the host's is: the one
/// `--chair` names, and no other.
#[test]
fn a_joined_chair_is_handed_over_by_the_seat_the_bridge_is_told() {
    let mut joined = input();
    joined["seat"] = serde_json::json!(1);
    let (_, chair) = parse(&joined.to_string(), "room", Some(1)).unwrap();
    assert_eq!(chair.seat, 1);
    assert!(parse(&joined.to_string(), "room", Some(0)).is_err());
    assert!(parse(&input().to_string(), "room", Some(1)).is_err());
}

#[test]
fn malformed_or_mismatched_handoffs_fail_without_echoing_tokens_or_json() {
    let mut bad = vec!["not-json-seat-secret".to_owned(), "x".repeat(17000)];
    for (key, value) in [
        ("seat", serde_json::json!(1)),
        ("game_id", serde_json::json!("other")),
        ("seat_token", serde_json::json!("")),
        ("session_token", serde_json::json!("session-secret\n")),
        ("unknown-session-secret", serde_json::json!(1)),
    ] {
        let mut value_in = input();
        value_in[key] = value;
        bad.push(value_in.to_string());
    }
    bad.push("{\"session_token\":\"session-secret\"}".to_owned());
    for line in bad {
        let err = parse(&line, "room", Some(0)).unwrap_err();
        assert!(!format!("{err:?}").contains("secret"));
    }
    assert!(parse(&input().to_string(), "other", Some(0)).is_err());
    assert!(parse(&input().to_string(), "room", Some(1)).is_err());
    assert!(parse(&input().to_string(), "room", None).is_err());
}

/// The room must be waiting, and the chair named the session's own
/// occupied person's chair: the host's chair 0, or a chair it joined.
#[test]
fn room_must_be_waiting_and_session_must_own_the_occupied_human_chair() {
    let value = serde_json::json!({"id":"room","state":"waiting","yours":true,"clock":{"decide_secs":0},"seats":[{"seat":0,"kind":"human","taken":true,"you":true},{"seat":1,"kind":"human","taken":true,"you":false}]});
    let room = |value: &serde_json::Value| serde_json::from_value(value.clone()).unwrap();
    validate_room(&room(&value), 0).unwrap();
    assert!(validate_room(&room(&value), 1).is_err(), "another's chair");
    assert!(validate_room(&room(&value), 2).is_err(), "no such chair");
    for key in ["you", "taken"] {
        let mut bad = value.clone();
        bad["seats"][0][key] = serde_json::json!(false);
        assert!(validate_room(&room(&bad), 0).is_err());
    }
    // A chair the account joined in a room another account hosts.
    let mut joined = value.clone();
    joined["yours"] = serde_json::json!(false);
    joined["seats"][0]["you"] = serde_json::json!(false);
    joined["seats"][1]["you"] = serde_json::json!(true);
    validate_room(&room(&joined), 1).unwrap();
    assert!(validate_room(&room(&joined), 0).is_err());
    let mut bad = value.clone();
    bad["state"] = serde_json::json!("playing");
    assert!(validate_room(&room(&bad), 0).is_err());
    let mut bad = value;
    bad["seats"][0]["kind"] = serde_json::json!("ai");
    assert!(validate_room(&room(&bad), 0).is_err());
}

#[tokio::test]
async fn existing_host_sit_only_reads_room_with_original_session() {
    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        let mut buf = [0; 1024];
        while !request.windows(4).any(|w| w == b"\r\n\r\n") {
            let n = stream.read(&mut buf).await.unwrap();
            assert!(n > 0);
            request.extend_from_slice(&buf[..n]);
        }
        let request = String::from_utf8(request).unwrap();
        assert!(request.starts_with("GET /lobby/games?q=room&limit=50 "));
        assert!(
            request
                .to_ascii_lowercase()
                .contains("authorization: bearer session-secret")
        );
        assert!(!request.contains("seat-secret"));
        let body = r#"{"games":[{"id":"room","state":"waiting","yours":true,"clock":{"decide_secs":0},"seats":[{"seat":0,"kind":"human","taken":true,"you":true}]}]}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(response.as_bytes()).await.unwrap();
    });
    let crate::Command::Join(join) = crate::Cli::try_parse_from([
        "baylee-seat",
        "join",
        "room",
        "--chair",
        "0",
        "--tethered",
        "--host-chair",
    ])
    .unwrap()
    .command
    else {
        panic!("not join")
    };
    let lobby = Lobby::new(&format!("http://{address}"));
    let (send, input_rx) = tokio::sync::oneshot::channel();
    send.send(input().to_string()).unwrap();
    let unstarted = Unstarted::default();
    let (by, chair) = sit(&join, &lobby, input_rx, &unstarted).await.unwrap();
    assert!(matches!(by, Standing::Guest(_)));
    assert_eq!(chair.seat_token, "seat-secret");
    assert!(unstarted.lock().unwrap().is_some());
    server.await.unwrap();
}
