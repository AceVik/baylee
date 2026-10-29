//! Real child-process cleanup tests; fixture commands are Rust test executables.
#![cfg(target_os = "linux")]
use baylee_train_ops::{
    files,
    guard::{self, Job},
    process,
};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::AtomicBool,
    time::Duration,
};

fn job(root: &Path, command: Vec<String>, timeout: f64) -> Job {
    Job {
        directory: root.join("job"),
        command,
        cwd: PathBuf::from(env!("CARGO_MANIFEST_DIR")),
        timeout_seconds: timeout,
        memory_gib: 1.0,
        inputs: vec![],
    }
}
fn fixture(name: &str) -> Vec<String> {
    vec![
        std::env::current_exe()
            .unwrap()
            .to_string_lossy()
            .into_owned(),
        "--ignored".into(),
        "--exact".into(),
        name.into(),
        "--nocapture".into(),
    ]
}
#[test]
#[ignore = "subprocess-only fixture"]
fn fixture_sleep() {
    std::thread::sleep(Duration::from_secs(30));
}

#[test]
#[ignore = "subprocess-only fixture"]
fn fixture_orphan() {
    let args = fixture("fixture_sleep");
    let child = Command::new(&args[0])
        .args(&args[1..])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    println!("OWNED_CHILD_PID={}", child.id());
    std::io::stdout().flush().unwrap();
    // Deliberately leave this child to exercise the guard, then exit its leader.
    std::process::exit(0);
}

#[test]
fn deadline_reaps_owned_group_and_preserves_failure() {
    let tmp = tempfile::tempdir().unwrap();
    let job = job(tmp.path(), fixture("fixture_sleep"), 0.2);
    assert!(!guard::run(&job, &AtomicBool::new(false)).unwrap());
    let receipt = files::read_json(&job.directory.join("job.json")).unwrap();
    assert_eq!(receipt["reason"], "deadline");
    assert_eq!(receipt["status"], "quarantined");
    let pid = u32::try_from(receipt["pid"].as_u64().unwrap()).unwrap();
    assert!(process::group_members(pid).unwrap().is_empty());
    assert!(process::identity(pid).is_none());
    assert!(guard::run(&job, &AtomicBool::new(false)).is_err());
}

#[test]
fn memory_limit_quarantines_and_reaps_real_process() {
    let tmp = tempfile::tempdir().unwrap();
    let mut job = job(tmp.path(), fixture("fixture_sleep"), 5.0);
    job.memory_gib = 0.000_001;
    assert!(!guard::run(&job, &AtomicBool::new(false)).unwrap());
    let receipt = files::read_json(&job.directory.join("job.json")).unwrap();
    assert_eq!(receipt["reason"], "memory_limit");
    let pid = u32::try_from(receipt["pid"].as_u64().unwrap()).unwrap();
    assert!(process::group_members(pid).unwrap().is_empty());
    assert!(process::identity(pid).is_none());
}

#[test]
fn exited_leader_does_not_leak_its_live_child() {
    let tmp = tempfile::tempdir().unwrap();
    let job = job(tmp.path(), fixture("fixture_orphan"), 5.0);
    assert!(guard::run(&job, &AtomicBool::new(false)).unwrap());
    let log = fs::read_to_string(job.directory.join("stdout.log")).unwrap();
    let child: u32 = log
        .lines()
        .find_map(|line| line.strip_prefix("OWNED_CHILD_PID="))
        .unwrap()
        .parse()
        .unwrap();
    assert!(process::identity(child).is_none());
}

#[test]
fn successful_job_reuse_requires_same_limits_and_sealed_inputs() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input");
    fs::write(&input, b"before").unwrap();
    let mut job = job(tmp.path(), vec!["true".into()], 5.0);
    job.inputs.push(input.clone());
    assert!(guard::run(&job, &AtomicBool::new(false)).unwrap());
    assert!(guard::run(&job, &AtomicBool::new(false)).unwrap());
    job.timeout_seconds = 6.0;
    assert!(guard::run(&job, &AtomicBool::new(false)).is_err());
    job.timeout_seconds = 5.0;
    fs::write(&input, b"changed").unwrap();
    assert!(guard::run(&job, &AtomicBool::new(false)).is_err());
}

#[test]
fn second_controller_cannot_take_owned_attempt() {
    let tmp = tempfile::tempdir().unwrap();
    let job = job(tmp.path(), vec!["true".into()], 5.0);
    let _lock = files::lock(&job.directory.join("guard.lock")).unwrap();
    assert!(guard::run(&job, &AtomicBool::new(false)).is_err());
    assert!(!job.directory.join("job.json").exists());
}
