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
fn run_house_shape_admits_runic_relics() {
    use baylee_tower::{
        cards::Formation,
        v2::catalog::{Definition, Family, Rarity, RunicRecipe},
    };
    // The calibrated ladder's builds carry relics of the current generation:
    // the outer shape check must not stop them before the store's own check.
    let runic = Definition {
        family: Family::Runic(RunicRecipe::from_index(0).expect("a first recipe")),
        rarity: Rarity::Common,
    };
    let mut house = input(json!({
        "action": "publish_house", "publisher": "22".repeat(32),
        "cohort": Uuid::now_v7(), "stage": 1, "archetype": 1,
        "build": {
            "formation": Formation { commander: baylee_core::ids::CardIndex::new(5), sideboard: vec![] },
            "deck": [], "active": [runic], "potions": [0, 0]
        }
    }));
    assert!(parses(&house));
    // A retained build of the third generation never had one.
    house["operation"]["build"]["formation"] = json!(null);
    assert!(!parses(&house));
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

#[test]
fn erase_accounts_takes_a_bounded_list_of_v7_accounts_and_nothing_else() {
    let erase =
        input(json!({ "action": "erase_accounts", "accounts": [Uuid::now_v7(), Uuid::now_v7()] }));
    assert!(parses(&erase));
    let mut extra = erase.clone();
    extra["operation"]["cohort"] = json!(Uuid::now_v7());
    assert!(!parses(&extra));
    let mut v4 = erase.clone();
    v4["operation"]["accounts"][1] = json!("2f1a6e3c-8b1d-4c7e-9a2b-3c4d5e6f7a8b");
    assert!(!parses(&v4));
    let mut empty = erase.clone();
    empty["operation"]["accounts"] = json!([]);
    assert!(!parses(&empty));
    let mut most = erase.clone();
    most["operation"]["accounts"] = json!(vec![Uuid::now_v7(); MAX_ERASED]);
    assert!(parses(&most));
    most["operation"]["accounts"] = json!(vec![Uuid::now_v7(); MAX_ERASED + 1]);
    assert!(!parses(&most));
}

#[test]
fn an_erasure_that_met_a_duel_under_way_is_not_complete() {
    let erased = |busy| Reply::Erased {
        removed: 1,
        absent: 1,
        busy,
    };
    assert!(erased(0).complete());
    assert!(!erased(1).complete());
    assert!(Reply::Done.complete());
}

#[test]
fn every_listed_action_is_one_this_build_parses() {
    for action in ACTIONS {
        let error = serde_json::from_value::<Operation>(json!({ "action": action }))
            .err()
            .map(|error| error.to_string())
            .unwrap_or_default();
        assert!(!error.contains("unknown variant"), "{action}: {error}");
    }
    let unknown = serde_json::from_value::<Operation>(json!({ "action": "erase_everything" }));
    assert!(unknown.is_err_and(|error| error.to_string().contains("unknown variant")));
}
