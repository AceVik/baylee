use super::*;
use crate::host::process::wire::{Contract, Initial};
use baylee_core::{ids::PlayerId, preset::LoopPolicy};
use std::path::PathBuf;

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let dir =
            std::env::temp_dir().join(format!("baylee-local-supervisor-{}", uuid::Uuid::now_v7()));
        std::fs::create_dir(&dir).unwrap();
        let fixture = Self(dir);
        std::fs::write(fixture.0.join("fixture.rs"), include_str!("fixture.rs")).unwrap();
        let result = Command::new("rustc")
            .arg("--edition=2024")
            .arg(fixture.0.join("fixture.rs"))
            .arg("-o")
            .arg(fixture.executable())
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        fixture
    }
    fn executable(&self) -> PathBuf {
        self.0
            .join(format!("fixture{}", std::env::consts::EXE_SUFFIX))
    }
    fn command(&self, bytes: &[u8]) -> Command {
        let path = self.0.join("reply");
        std::fs::write(&path, bytes).unwrap();
        let mut command = Command::new(self.executable());
        command.env("BAYLEE_FIXTURE_REPLY", path);
        command
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn opening() -> Bootstrap {
    let mut preset = crate::host::tests::duel_preset();
    preset.house_rules.loop_policy = LoopPolicy::Off;
    Bootstrap {
        contract: Contract::new(LoopPolicy::Off),
        initial: Initial::New(serde_json::to_vec(&preset).unwrap()),
        seat: PlayerId::new(0),
        names: vec!["One".into(), "Two".into()],
        persist: false,
    }
}

fn reply(mode: &str) -> Vec<u8> {
    if mode == "startup" {
        return Vec::new();
    }
    if mode == "invalid" {
        return vec![255; 4];
    }
    let mut hello = Contract::new(LoopPolicy::Off);
    if mode == "version" {
        hello.view += 1;
    }
    let mut bytes = Vec::new();
    for value in [
        Output::Hello(hello),
        Output::Progress(Reply {
            messages: Vec::new(),
            record: Vec::new(),
            resume: None,
            done: mode == "idle",
        }),
    ] {
        baylee_worker::frame::write(&mut bytes, &baylee_core::wire::encode(&value), FRAME_LIMIT)
            .unwrap();
    }
    bytes
}

async fn case(fixture: &Fixture, mode: &str) -> Result<(), String> {
    let (stop, stopped) = oneshot::channel();
    let (_input_tx, input) = mpsc::channel(1);
    let (output, _output_rx) = mpsc::channel(1);
    let pid = AtomicU32::new(0);
    let limits = Limits {
        startup: Duration::from_millis(400),
        operation: Duration::from_millis(200),
        resident_bytes: if mode == "memory" { 1 } else { u64::MAX },
    };
    let running = run(
        fixture.command(&reply(mode)),
        opening(),
        input,
        &output,
        stopped,
        &pid,
        limits,
    );
    tokio::pin!(running);
    let result = if mode == "idle" {
        tokio::select! {
            result = &mut running => panic!("idle thinking was interrupted: {result:?}"),
            () = tokio::time::sleep(Duration::from_millis(700)) => {}
        }
        drop(stop);
        running.await
    } else {
        tokio::select! {
            result = &mut running => result,
            () = tokio::time::sleep(Duration::from_secs(5)) => {
                drop(stop);
                running.await.unwrap();
                panic!("supervisor did not enforce {mode}");
            }
        }
    };
    let pid = sysinfo::Pid::from_u32(pid.load(Ordering::Acquire));
    let mut system = sysinfo::System::new().unwrap();
    system.refresh_processes_specifics(
        sysinfo::ProcessesToUpdate::Some(&[pid]),
        true,
        sysinfo::ProcessRefreshKind::nothing(),
    );
    assert!(
        system.process(pid).is_none(),
        "the {mode} child was not reaped"
    );
    result
}

#[tokio::test]
async fn private_workers_enforce_bounds_version_and_reaping_but_allow_idle_thinking() {
    let fixture = Fixture::new();
    for (mode, expected) in [
        ("startup", "initialization exceeded"),
        ("busy", "operation exceeded"),
        ("invalid", "response was too large"),
        ("version", "version differs"),
        ("memory", "resident-memory budget"),
    ] {
        let reason = case(&fixture, mode).await.unwrap_err();
        assert!(reason.contains(expected), "{mode}: {reason}");
    }
    case(&fixture, "idle").await.unwrap();
}
