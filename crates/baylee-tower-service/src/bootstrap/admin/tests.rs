use super::*;
use serde_json::json;

fn input(operation: serde_json::Value) -> serde_json::Value {
    let mut input = json!({ "configuration": {
        "database_url": "postgres://qa@127.0.0.1:1/synthetic", "schema": "tower_test",
        "development": true, "service": null
    } });
    input["operation"] = operation;
    input
}

fn parses(value: &serde_json::Value) -> bool {
    Input::parse(&serde_json::to_vec(value).unwrap()).is_ok()
}

#[test]
fn offline_commands_are_closed_bounded_and_separate_from_serving() {
    let provision = input(
        json!({ "action": "provision", "name": "synthetic publisher",
        "role": "publisher", "credential": "11".repeat(32) }),
    );
    assert!(parses(&provision));
    for field in ["configuration", "operation"] {
        let mut extra = provision.clone();
        extra[field]["unknown"] = json!(true);
        assert!(!parses(&extra));
    }
    for name in ["", " padded", "line\nbreak"] {
        let mut invalid = provision.clone();
        invalid["operation"]["name"] = json!(name);
        assert!(!parses(&invalid));
    }
    for role in ["administrator", "gateway", ""] {
        let mut invalid = provision.clone();
        invalid["operation"]["role"] = json!(role);
        assert!(!parses(&invalid));
    }
    let mut invalid = provision.clone();
    invalid["operation"]["credential"] = json!("PRIVATE_CREDENTIAL_CANARY");
    assert!(!parses(&invalid));
    invalid = provision.clone();
    invalid["configuration"]["service"] = json!({
        "listen": "127.0.0.1:0", "origin": "http://127.0.0.1:29771",
        "central": "http://127.0.0.1:29772", "record_bytes": 1024, "workers": []
    });
    assert!(
        !parses(&invalid),
        "administration cannot also start a listener"
    );
    invalid = provision;
    invalid["configuration"]["database_url"] = json!("postgres://qa@remote.example/synthetic");
    assert!(!parses(&invalid));
    assert!(Input::parse(&vec![b' '; MAX_INPUT + 1]).is_err());
}

#[test]
fn public_ids_origins_and_house_bounds_are_checked_before_connecting() {
    let id = Uuid::now_v7();
    for action in [
        json!({ "action": "revoke", "operator": id }),
        json!({ "action": "pause_enrollment", "cohort": id }),
        json!({ "action": "approve_host", "host": id, "origin": "http://127.0.0.1:29773" }),
    ] {
        assert!(parses(&input(action)));
    }
    for action in [
        json!({ "action": "revoke", "operator": Uuid::nil() }),
        json!({ "action": "pause_enrollment", "cohort": Uuid::nil() }),
        json!({ "action": "approve_host", "host": id, "origin": "http://remote.example" }),
        json!({ "action": "approve_host", "host": id, "origin": "http://127.0.0.1:29773/path" }),
        json!({ "action": "execute_sql", "statement": "SELECT 1" }),
    ] {
        assert!(!parses(&input(action)));
    }
    let mut house = input(
        json!({ "action": "publish_house", "publisher": "22".repeat(32),
        "cohort": id, "stage": 1, "archetype": 1,
        "build": { "deck": [], "active": [], "potions": [0, 0] } }),
    );
    // Deck/catalog coverage is intentionally checked with the saved publication,
    // not claimed by this offline shape check.
    assert!(parses(&house));
    house["operation"]["stage"] = json!(0);
    assert!(!parses(&house));
    house["operation"]["stage"] = json!(MAX_STAGES);
    // One past the widest cohort a publication may freeze (migration 016).
    house["operation"]["archetype"] = json!(65);
    assert!(!parses(&house));
    house["operation"]["archetype"] = json!(3);
    house["operation"]["build"]["potions"] = json!([2, 1]);
    assert!(!parses(&house));
}

#[test]
fn run_house_shape_admits_six_slots_but_retained_builds_keep_their_limits() {
    use baylee_tower::{
        cards::Formation,
        v2::catalog::{Definition, Family, Rarity},
    };
    let item = Definition {
        family: Family::Grove,
        rarity: Rarity::Common,
    };
    let mut house = input(json!({
        "action": "publish_house", "publisher": "22".repeat(32),
        "cohort": Uuid::now_v7(), "stage": 12, "archetype": 8,
        "build": {
            "formation": Formation { commander: baylee_core::ids::CardIndex::new(5), sideboard: vec![] },
            "deck": [], "active": vec![item; 6], "potions": [1, 1]
        }
    }));
    assert!(parses(&house));
    house["operation"]["build"]["active"] = json!(vec![item; 7]);
    assert!(!parses(&house));
    house["operation"]["build"]["active"] = json!(vec![item; 4]);
    house["operation"]["build"]["formation"] = json!(null);
    assert!(!parses(&house));
    house["operation"]["build"]["active"] = json!(vec![item; 3]);
    assert!(parses(&house));
}

#[test]
fn supervision_is_three_closed_transitions_on_one_run() {
    let run = Uuid::now_v7();
    let supervise = |recovery: serde_json::Value| {
        input(
            json!({ "action": "supervise", "supervisor": "33".repeat(32),
            "run": run, "revision": 4, "recovery": recovery }),
        )
    };
    for recovery in [
        json!({ "kind": "quarantine" }),
        json!({ "kind": "interrupt" }),
        json!({ "kind": "resume", "host": null }),
        json!({ "kind": "resume", "host": Uuid::now_v7() }),
    ] {
        assert!(parses(&supervise(recovery)));
    }
    for recovery in [
        json!({ "kind": "resume", "host": Uuid::nil() }),
        json!({ "kind": "settle", "outcome": "victory" }),
        json!({ "kind": "quarantine", "gold": 99 }),
    ] {
        assert!(!parses(&supervise(recovery)));
    }
    let mut nil = supervise(json!({ "kind": "interrupt" }));
    nil["operation"]["run"] = json!(Uuid::nil());
    assert!(!parses(&nil));
}

#[test]
fn modes_never_accept_credentials_or_extra_arguments() {
    use super::super::{Mode, mode};
    for (name, expected) in [
        ("--admin", Mode::Admin),
        ("--check-admin", Mode::CheckAdmin),
    ] {
        assert!(mode([name.to_owned()].into_iter()).is_ok_and(|actual| actual == expected));
        assert!(mode([name.to_owned(), "PRIVATE_ARGUMENT_CANARY".into()].into_iter()).is_err());
    }
    assert!(mode(["--admin=PRIVATE_ARGUMENT_CANARY".into()].into_iter()).is_err());
}
