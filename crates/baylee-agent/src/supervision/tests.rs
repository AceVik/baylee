use super::*;
use std::{process::Stdio, time::Duration};

#[path = "../../tests/support/mod.rs"]
mod support;

#[tokio::test]
async fn stalled_and_oversized_workers_are_reaped_but_idle_players_are_not_timed_out() {
    let temp = tempfile::tempdir().unwrap();
    let executable = support::executable(temp.path());
    for (mode, expected) in [
        ("start-stall", "initialization exceeded"),
        ("busy", "operation exceeded"),
        ("invalid-progress", "progress stream was invalid"),
        ("memory", "resident-memory budget"),
        ("idle", "stopped"),
    ] {
        std::fs::write(temp.path().join("mode"), mode).unwrap();
        let mut command = tokio::process::Command::new(&executable);
        command
            .env_remove(baylee_agent::AGENT_TOKEN_VAR)
            .env("BAYLEE_TEST_CHILD_PID", temp.path().join("child.pid"))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        let child = command.spawn().unwrap();
        let pid = child.id().unwrap();
        let running: Running = std::sync::Arc::default();
        let (stop, stopped) = oneshot::channel();
        running.lock().unwrap().insert(
            mode.into(),
            crate::OwnedEngine {
                execution: support::identity(),
                stop: Some(stop),
            },
        );
        let (out, mut reported) = mpsc::unbounded_channel();
        let task = tokio::spawn(with_limits(
            child,
            stopped,
            running.clone(),
            out,
            mode.into(),
            baylee_worker::Limits {
                // Only the stalled start is meant to run out of this budget.
                // The others must get past it on a machine busy with the
                // rest of the suite, where a process start can take longer
                // than the 300 ms that are enough to show a stall.
                startup: if mode == "start-stall" {
                    Duration::from_millis(300)
                } else {
                    Duration::from_secs(3)
                },
                operation: Duration::from_millis(100),
                resident_bytes: if mode == "memory" { 1 } else { u64::MAX },
            },
        ));
        if mode == "idle" {
            tokio::time::sleep(Duration::from_millis(600)).await;
            assert!(
                reported.try_recv().is_err(),
                "idle thinking must not time out"
            );
            assert!(running.lock().unwrap().contains_key(mode));
            crate::stop(&running, mode);
        }
        tokio::time::timeout(Duration::from_secs(5), task)
            .await
            .unwrap()
            .unwrap();
        assert!(running.lock().unwrap().is_empty());
        let message = reported.recv().await.unwrap();
        let Some(v1::envelope::Msg::EngineStatus(status)) = message.msg else {
            panic!("an owned process status");
        };
        assert!(
            status.detail.contains(expected),
            "{mode}: {}",
            status.detail
        );
        assert_eq!(status.kind, v1::engine_status::Kind::Exited as i32);
        #[cfg(unix)]
        {
            let exists = std::process::Command::new("/bin/kill")
                .args(["-0", &pid.to_string()])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .unwrap();
            assert!(!exists.success(), "{mode}: the owned child was not reaped");
        }
        #[cfg(not(unix))]
        let _ = pid;
    }
}
