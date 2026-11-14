use super::*;
use serde_json::{Value, json};

fn development() -> Value {
    json!({ "database_url": "postgres://synthetic:synthetic@127.0.0.1:29870/tower", "schema": "synthetic_tower", "development": true,
        "service": { "listen": "127.0.0.1:29871", "origin": "http://127.0.0.1:29871", "central": "http://127.0.0.1:29872", "record_bytes": 1_048_576, "workers": [] } })
}

fn production() -> Value {
    let mut value = development();
    value["development"] = json!(false);
    value["database_url"] = json!(
        "postgres://synthetic:synthetic@db.example.invalid/tower?sslmode=verify-full&sslrootcert=/tmp/ca.pem"
    );
    value["service"]["origin"] = json!("https://tower.example.invalid");
    value["service"]["central"] = Value::Null;
    value["service"]["workers"] = json!([{ "cohort": Uuid::now_v7(), "artifact": "/tmp/synthetic-engine", "verifier": "01".repeat(32), "supervisor": "02".repeat(32) }]);
    value
}

fn parse(value: &Value) -> bool {
    Config::parse(&serde_json::to_vec(value).unwrap()).is_ok()
}

#[test]
fn bootstrap_configuration_rejects_unbounded_and_cross_environment_authority() {
    assert!(parse(&development()));
    let mut ipv6 = development();
    ipv6["database_url"] = json!("postgres://synthetic:synthetic@[::1]:29870/tower");
    assert!(parse(&ipv6));
    assert!(parse(&production()));
    assert!(Config::parse(&vec![b' '; MAX_CONFIG + 1]).is_err());
    for (pointer, replacement) in [
        (
            "/database_url",
            json!("postgres://synthetic:synthetic@db.example.invalid/tower"),
        ),
        (
            "/database_url",
            json!("postgres://synthetic:synthetic@localhost/tower"),
        ),
        (
            "/database_url",
            json!("postgres://synthetic:synthetic@127.1/tower"),
        ),
        (
            "/database_url",
            json!("postgres://synthetic:synthetic@127.0.0.1/tower?host=remote.invalid"),
        ),
        ("/schema", json!("bad;schema")),
        ("/service/listen", json!("0.0.0.0:29871")),
        ("/service/central", json!(central_api::LIVE_ORIGIN)),
        ("/service/origin", json!("https://tower.example.invalid")),
        ("/service/origin", json!("http://127.0.0.1:29872")),
        ("/service/record_bytes", json!(MAX_RECORD_BYTES + 1)),
    ] {
        let mut bad = development();
        *bad.pointer_mut(pointer).unwrap() = replacement;
        assert!(!parse(&bad), "unsafe development setting accepted");
    }
    for url in [
        "postgres://synthetic:synthetic@db.example.invalid/tower",
        "postgres://synthetic:synthetic@db.example.invalid/tower?sslmode=require&sslrootcert=/tmp/ca.pem",
        "postgres://synthetic:synthetic@db.example.invalid/tower?sslmode=verify-full&sslrootcert=relative.pem",
        "postgres://synthetic:synthetic@db.example.invalid/tower?sslmode=verify-full&sslrootcert=/tmp/ca.pem&host=other.invalid",
    ] {
        let mut bad = production();
        bad["database_url"] = json!(url);
        assert!(!parse(&bad));
    }
    for (pointer, replacement) in [
        ("/service/workers", json!([])),
        ("/service/central", json!("http://127.0.0.1:29872")),
        ("/service/workers/0/artifact", json!("relative-engine")),
        ("/service/workers/0/verifier", json!("untrusted-key")),
    ] {
        let mut bad = production();
        *bad.pointer_mut(pointer).unwrap() = replacement;
        assert!(!parse(&bad), "unsafe production setting accepted");
    }
    let mut duplicate = production();
    let worker = duplicate["service"]["workers"][0].clone();
    duplicate["service"]["workers"]
        .as_array_mut()
        .unwrap()
        .push(worker);
    assert!(!parse(&duplicate));
    let mut extended = development();
    extended["skip_auth"] = json!(true);
    assert!(!parse(&extended));
}
