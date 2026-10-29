use super::*;

fn setup() -> (tempfile::TempDir, Value) {
    let dir = tempfile::tempdir().unwrap();
    let config = json!({"thread":"01a16325-d29f-7ad1-9625-cbb5bf2cb1a4","poll_seconds":1,"training":{"root":dir.path().join("run"),"controller":{"pid":1234,"start_ticks":567,"boot_id":"boot"},"round_floor":9}});
    files::atomic_json(&dir.path().join("config.json"), &config).unwrap();
    (dir, config)
}

#[test]
fn archived_identity_token_is_unchanged() {
    assert_eq!(token(&json!({"boot_id":"ebf55ae5-05a2-41fc-a994-e5da27268ced","pid":1_161_477,"start_ticks":5_439_662})).unwrap(),"49efb950e5b82472");
}

#[test]
fn complete_rounds_dedupe_and_queue_retries_preserve_id() {
    let (dir, config) = setup();
    let root = dir.path();
    let run = root.join("run");
    files::atomic_json(
        &run.join("rounds/round-0009/summary.json"),
        &json!({"round":9,"completed_unix":12.0,"mixed_stage":{"model":"checkpoint"}}),
    )
    .unwrap();
    files::atomic_json(&run.join("status.json"), &json!({"phase":"training"})).unwrap();
    let owner = identity(&config["training"]["controller"]).unwrap();
    let current = |_| Some(owner.clone());
    let mut calls = 0;
    check_with(root, 100.0, current, |_, _| {
        calls += 1;
        anyhow::bail!("offline")
    })
    .unwrap();
    let old = files::read_json(&root.join("state.json")).unwrap();
    let id = old["pending"]
        .as_object()
        .unwrap()
        .keys()
        .next()
        .unwrap()
        .clone();
    check_with(root, 101.0, current, |_, _| {
        calls += 1;
        Ok("unexpected".into())
    })
    .unwrap();
    assert_eq!(calls, 1);
    check_with(root, 161.0, current, |_, message| {
        calls += 1;
        assert!(message.contains(&id));
        Ok("queued".into())
    })
    .unwrap();
    check_with(root, 163.0, current, |_, _| {
        calls += 1;
        Ok("duplicate".into())
    })
    .unwrap();
    assert_eq!(calls, 2);
    let state = files::read_json(&root.join("state.json")).unwrap();
    assert!(state["pending"].as_object().unwrap().is_empty());
    assert!(state["delivered"].get(&id).is_some());
}

#[test]
fn recycled_pid_and_corrupt_ledgers_never_hide_failure_or_redeliver() {
    let (dir, _) = setup();
    let root = dir.path();
    let wrong = |_| {
        Some(Identity {
            pid: 1234,
            start_ticks: 999,
            boot_id: "boot".into(),
        })
    };
    check_with(root, 100.0, wrong, |_, _| panic!("needs confirmation")).unwrap();
    let mut calls = 0;
    check_with(root, 102.0, wrong, |_, m| {
        calls += 1;
        assert!(m.contains("exited"));
        Ok("receipt".into())
    })
    .unwrap();
    assert_eq!(calls, 1);
    files::atomic_json(&root.join("state.json"), &json!({"delivered":[]})).unwrap();
    assert!(check_with(root, 200.0, wrong, |_, _| panic!("corrupt ledger")).is_err());
}

#[test]
fn owner_stop_and_future_quota_hold_are_not_training_restarts() {
    let (dir, mut config) = setup();
    config.as_object_mut().unwrap().remove("training");
    config["training_stopped_by_owner"] = json!({"phase":"stopped"});
    config["flash_probe"] = json!({"root":dir.path().join("probe"),"not_before_unix":1000.0});
    files::atomic_json(&dir.path().join("config.json"), &config).unwrap();
    check_with(
        dir.path(),
        100.0,
        |_| None,
        |_, _| panic!("no premature quota wake"),
    )
    .unwrap();
    let state = files::read_json(&dir.path().join("state.json")).unwrap();
    assert!(state["observed"].as_object().unwrap().is_empty());
}
