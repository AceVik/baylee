use super::*;
use std::fs;

fn setup() -> (tempfile::TempDir, Identity) {
    let temp = tempfile::tempdir().unwrap();
    let identity = process::identity(std::process::id()).unwrap();
    let config = json!({"root":temp.path(),"thread":"019a18f5-1111-7111-8111-111111111111","controller":identity,"success_phases":["complete"],"artifacts":["paired.json"],"poll_seconds":30,"error_polls":3,"death_polls":2});
    arm(temp.path(), &config).unwrap();
    files::atomic_json(
        &temp.path().join("status.json"),
        &json!({"phase":"training"}),
    )
    .unwrap();
    (temp, identity)
}

#[test]
fn success_is_durable_and_never_delivered_twice() {
    let (temp, id) = setup();
    let root = temp.path();
    let mut calls = 0;
    files::atomic_json(&root.join("status.json"), &json!({"phase":"complete"})).unwrap();
    files::atomic_json(&root.join("paired.json"), &json!({})).unwrap();
    for time in [30.0, 60.0] {
        assert_eq!(
            check_with(
                root,
                time,
                |_| Some(id.clone()),
                |thread, message| {
                    calls += 1;
                    assert!(thread.starts_with("019a"));
                    assert!(message.contains("completed"));
                    Ok("receipt".into())
                }
            )
            .unwrap(),
            "delivered"
        );
    }
    assert_eq!(calls, 1);
    assert_eq!(
        files::read_json(&root.join("watchdog/delivered.json")).unwrap()["queue_receipt"],
        "receipt"
    );
}

#[test]
fn recycled_pid_requires_confirmation_and_artifact_completion() {
    let (temp, id) = setup();
    let root = temp.path();
    let mut recycled = id.clone();
    recycled.start_ticks += 1;
    assert_eq!(
        check_with(
            root,
            30.0,
            |_| Some(recycled.clone()),
            |_, _| panic!("too early")
        )
        .unwrap(),
        "confirming-exit"
    );
    assert_eq!(
        check_with(root, 31.0, |_| None, |_, _| panic!("throttled")).unwrap(),
        "waiting"
    );
    files::atomic_json(&root.join("status.json"), &json!({"phase":"complete"})).unwrap();
    let mut message = String::new();
    assert_eq!(
        check_with(
            root,
            60.0,
            |_| Some(recycled.clone()),
            |_, m| {
                message = m.to_owned();
                Ok("ok".into())
            }
        )
        .unwrap(),
        "delivered"
    );
    assert!(message.contains("incomplete terminal result"));
}

#[test]
fn queue_retry_preserves_event_and_backoff() {
    let (temp, id) = setup();
    let root = temp.path();
    files::atomic_json(&root.join("status.json"), &json!({"phase":"failed"})).unwrap();
    let mut first = String::new();
    assert_eq!(
        check_with(
            root,
            30.0,
            |_| Some(id.clone()),
            |_, m| {
                first = m.to_owned();
                anyhow::bail!("test queue unavailable")
            }
        )
        .unwrap(),
        "queue-retry"
    );
    assert_eq!(
        check_with(root, 60.0, |_| None, |_, _| panic!("backoff")).unwrap(),
        "backoff"
    );
    assert_eq!(
        check_with(
            root,
            90.0,
            |_| None,
            |_, m| {
                assert_eq!(m, first);
                Ok("ok".into())
            }
        )
        .unwrap(),
        "delivered"
    );
}

#[test]
fn corrupt_state_never_erases_delivery_history_and_busy_is_quiet() {
    let (temp, id) = setup();
    let root = temp.path();
    let lock = files::lock(&root.join("watchdog/lock")).unwrap();
    assert_eq!(
        check_with(root, 30.0, |_| Some(id.clone()), |_, _| panic!("busy")).unwrap(),
        "busy"
    );
    drop(lock);
    fs::write(root.join("watchdog/state.json"), b"{").unwrap();
    assert!(check_with(root, 30.0, |_| Some(id.clone()), |_, _| panic!("corrupt")).is_err());
    assert_eq!(fs::read(root.join("watchdog/state.json")).unwrap(), b"{");
}

#[test]
fn status_errors_have_grace_and_recovery_resets_count() {
    let (temp, id) = setup();
    let root = temp.path();
    fs::remove_file(root.join("status.json")).unwrap();
    for time in [30.0, 60.0] {
        assert_eq!(
            check_with(root, time, |_| Some(id.clone()), |_, _| panic!("grace")).unwrap(),
            "running"
        );
    }
    files::atomic_json(&root.join("status.json"), &json!({"phase":"training"})).unwrap();
    assert_eq!(
        check_with(root, 90.0, |_| Some(id.clone()), |_, _| panic!("recovered")).unwrap(),
        "running"
    );
    fs::write(root.join("status.json"), b"[]").unwrap();
    for time in [120.0, 150.0] {
        assert_eq!(
            check_with(root, time, |_| Some(id.clone()), |_, _| panic!("new grace")).unwrap(),
            "running"
        );
    }
    assert_eq!(
        check_with(
            root,
            180.0,
            |_| Some(id.clone()),
            |_, m| {
                assert!(m.contains("persistent unreadable"));
                Ok("ok".into())
            }
        )
        .unwrap(),
        "delivered"
    );
}

#[test]
fn proc_names_with_parentheses_and_zombies_parse_correctly() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join("12")).unwrap();
    fs::create_dir_all(root.join("sys/kernel/random")).unwrap();
    fs::write(root.join("sys/kernel/random/boot_id"), "boot\n").unwrap();
    let mut fields = vec!["0"; 20];
    fields[0] = "S";
    fields[19] = "1234";
    fs::write(
        root.join("12/stat"),
        format!("12 (name with ) spaces) {}", fields.join(" ")),
    )
    .unwrap();
    assert_eq!(
        process::identity_at(12, root),
        Some(Identity {
            pid: 12,
            boot_id: "boot".into(),
            start_ticks: 1234
        })
    );
    fields[0] = "Z";
    fs::write(
        root.join("12/stat"),
        format!("12 (zombie) {}", fields.join(" ")),
    )
    .unwrap();
    assert!(process::identity_at(12, root).is_none());
}
