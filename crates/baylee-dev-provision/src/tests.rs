use super::{central, parse, signin};

/// The sign-in proof is for the stack's own Central: any other origin is
/// refused before a byte leaves the machine, and the refusal repeats nothing
/// of the request.
#[test]
fn the_sign_in_proof_goes_to_a_loopback_central_only() {
    let secret = "correct-horse-battery-staple";
    for endpoint in ["https://central.baylee.app", "http://example.org:28790", ""] {
        let request = serde_json::json!({
            "endpoint": endpoint,
            "username": "dev_admin",
            "password": secret,
        });
        let input = parse::<signin::Input>(&serde_json::to_vec(&request).unwrap()).unwrap();
        let error = signin::run(input)
            .err()
            .expect("not a development origin")
            .to_string();
        assert!(error.contains("loopback"), "{error}");
        assert!(!error.contains(secret), "{error}");
    }
}

/// The first document is what the client reads back after its first sign-in:
/// the stack's gateway and nothing borrowed from production.
#[test]
fn the_first_document_names_the_stacks_gateway_only() {
    let text = String::from_utf8(central::first_document("http://127.0.0.1:28766")).unwrap();
    let value: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(
        value["gateways"],
        serde_json::json!(["http://127.0.0.1:28766"])
    );
    assert_eq!(value["version"], 1);
    assert!(value["legacy"].is_null());
    assert!(!text.contains("baylee.app") && !text.contains("acevik"));
}

/// A refused request says where it broke and repeats none of it: the request
/// is made of passwords and credentials.
#[test]
fn a_refused_request_does_not_echo_its_secrets() {
    let secret = "correct-horse-battery-staple";
    let request = format!(r#"{{"postgres_port":"{secret}"}}"#);
    let error = parse::<central::Input>(request.as_bytes())
        .err()
        .expect("a port is a number")
        .to_string();
    assert!(!error.contains(secret), "{error}");
    assert!(error.contains("line 1"), "{error}");

    let unknown = format!(r#"{{"surprise":"{secret}"}}"#);
    let error = parse::<central::Input>(unknown.as_bytes())
        .err()
        .expect("unknown fields are refused")
        .to_string();
    assert!(!error.contains(secret), "{error}");
}
