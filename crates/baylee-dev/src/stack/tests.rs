use super::{DEFAULT_MONITOR, StackState, loopback_http_origin, services::parse_http_200_body};
use std::path::PathBuf;

#[test]
fn default_interactive_monitor_is_left_2k_index_two() {
    assert_eq!(DEFAULT_MONITOR, 2);
}

#[test]
fn loopback_http_origin_accepts_only_numeric_ipv4_loopback_with_nonzero_port() {
    assert!(loopback_http_origin("http://127.0.0.1:28790"));
    assert!(loopback_http_origin("http://127.0.0.1:28794"));
    for bad in [
        "https://baylee.app",
        "https://127.0.0.1:28790",
        "http://localhost:28790",
        "http://127.0.0.1:0",
        "http://127.0.0.1:28790/",
        "http://127.0.0.1:28790/v1",
        "http://192.168.0.53:28790",
    ] {
        assert!(!loopback_http_origin(bad), "{bad}");
    }
}

#[test]
fn stack_state_json_contains_no_secret_material() {
    let state = StackState {
        supervisor_pid: 42,
        postgres_port: 25432,
        gateway_port: 28766,
        admin_port: 28767,
        feedback_port: 28780,
        gateway_origin: "http://127.0.0.1:28766".into(),
        central_endpoint: Some("http://127.0.0.1:28790".into()),
        monitor: 2,
        logs_dir: PathBuf::from("target/local-stack/logs"),
        client_pid: Some(99),
    };
    let json = serde_json::to_string(&state).unwrap();
    for forbidden in ["password", "token", "secret", "postgres://"] {
        assert!(
            !json.contains(forbidden),
            "unexpected {forbidden} in {json}"
        );
    }
    let decoded: StackState = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, state);
}

#[test]
fn parse_http_200_body_extracts_ok_payload_and_rejects_errors() {
    let ok = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"ok\":true}";
    assert_eq!(parse_http_200_body(ok).as_deref(), Some("{\"ok\":true}"));
    let err = b"HTTP/1.1 503 Service Unavailable\r\n\r\n{\"ok\":false}";
    assert_eq!(parse_http_200_body(err), None);
}

#[test]
fn a_second_supervisor_cannot_replace_data_or_take_ownership() {
    let root =
        std::env::temp_dir().join(format!("baylee-stack-{}", crate::secret::hex(12).unwrap()));
    std::fs::create_dir(&root).unwrap();
    let result = (|| -> anyhow::Result<()> {
        let guard = super::lock_stack(&root)?;
        assert!(super::lock_stack(&root).is_err());
        let data = root.join("pgdata");
        super::postgres::claim_data_dir(&data)?;
        std::fs::write(data.join("sentinel"), b"existing data")?;
        assert!(super::postgres::claim_data_dir(&data).is_err());
        assert_eq!(std::fs::read(data.join("sentinel"))?, b"existing data");
        drop(guard);
        let _guard = super::lock_stack(&root)?;
        Ok(())
    })();
    std::fs::remove_dir_all(&root).unwrap();
    result.unwrap();
}
