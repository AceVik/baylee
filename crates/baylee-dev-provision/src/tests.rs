use super::{central, parse};

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
