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
#[ignore = "subprocess-only fixture"]
fn fixture_invocation_identity() {
    let invocation = PathBuf::from(std::env::args_os().next().unwrap());
    assert_eq!(invocation.file_name().unwrap(), "virtual-runtime");
    println!("INVOCATION={}", invocation.display());
    println!("TARGET={}", std::env::current_exe().unwrap().display());
}

#[test]
fn symlink_invocation_preserves_runtime_identity_and_reuse_binds_target() {
    let tmp = tempfile::tempdir().unwrap();
    let alias = tmp.path().join("virtual-runtime");
    let target = std::env::current_exe().unwrap().canonicalize().unwrap();
    std::os::unix::fs::symlink(&target, &alias).unwrap();
    let mut command = fixture("fixture_invocation_identity");
    command[0] = alias.display().to_string();
    let job = job(tmp.path(), command, 5.0);
    assert!(guard::run(&job, &AtomicBool::new(false)).unwrap());
    let receipt = files::read_json(&job.directory.join("job.json")).unwrap();
    assert_eq!(
        receipt["executable_invocation"],
        alias.display().to_string()
    );
    assert_eq!(receipt["executable"], target.display().to_string());
    assert_eq!(
        receipt["executable_sha256"],
        files::sha256(&target).unwrap()
    );
    let log = fs::read_to_string(job.directory.join("stdout.log")).unwrap();
    assert!(log.contains(&format!("INVOCATION={}", alias.display())));
    assert!(guard::run(&job, &AtomicBool::new(false)).unwrap());
    let identical = tmp.path().join("same-bytes-different-target");
    fs::copy(&target, &identical).unwrap();
    fs::remove_file(&alias).unwrap();
    std::os::unix::fs::symlink(&identical, &alias).unwrap();
    assert_eq!(
        files::sha256(&identical).unwrap(),
        receipt["executable_sha256"]
    );
    assert!(guard::run(&job, &AtomicBool::new(false)).is_err());
}

#[test]
#[ignore = "subprocess-only fixture"]
fn fixture_retarget_invocation() {
    let invocation = PathBuf::from(std::env::args_os().next().unwrap());
    let replacement = invocation.parent().unwrap().join("replacement-runtime");
    fs::remove_file(&invocation).unwrap();
    std::os::unix::fs::symlink(replacement, invocation).unwrap();
}

#[test]
fn invocation_retargeted_during_execution_is_quarantined_even_with_identical_bytes() {
    let tmp = tempfile::tempdir().unwrap();
    let target = std::env::current_exe().unwrap().canonicalize().unwrap();
    let alias = tmp.path().join("virtual-runtime");
    fs::copy(&target, tmp.path().join("replacement-runtime")).unwrap();
    std::os::unix::fs::symlink(&target, &alias).unwrap();
    let mut command = fixture("fixture_retarget_invocation");
    command[0] = alias.display().to_string();
    let job = job(tmp.path(), command, 5.0);
    assert!(!guard::run(&job, &AtomicBool::new(false)).unwrap());
    let receipt = files::read_json(&job.directory.join("job.json")).unwrap();
    assert_eq!(receipt["status"], "quarantined");
    assert_eq!(receipt["reason"], "input_changed");
    assert_eq!(receipt["returncode"], 0);
    let pid = u32::try_from(receipt["pid"].as_u64().unwrap()).unwrap();
    assert!(process::identity(pid).is_none());
    assert!(guard::run(&job, &AtomicBool::new(false)).is_err());
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

fn repository(root: &Path) {
    fs::create_dir_all(root).unwrap();
    for args in [
        vec!["init", "-q"],
        vec![
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "-qm",
            "fixture",
            "--allow-empty",
        ],
    ] {
        assert!(
            Command::new("git")
                .args(args)
                .current_dir(root)
                .status()
                .unwrap()
                .success()
        );
    }
}

#[test]
fn completed_job_reuse_rejects_a_changed_checkout() {
    let tmp = tempfile::tempdir().unwrap();
    let cwd = tmp.path().join("source");
    repository(&cwd);
    let mut job = job(tmp.path(), vec!["true".into()], 5.0);
    job.cwd = cwd.clone();
    assert!(guard::run(&job, &AtomicBool::new(false)).unwrap());
    assert!(guard::run(&job, &AtomicBool::new(false)).unwrap());
    fs::write(cwd.join("worker.rs"), b"// changed training code\n").unwrap();
    assert!(
        Command::new("git")
            .args(["add", "worker.rs"])
            .current_dir(&cwd)
            .status()
            .unwrap()
            .success()
    );
    assert!(guard::run(&job, &AtomicBool::new(false)).is_err());
    assert!(
        Command::new("git")
            .args([
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.invalid",
                "commit",
                "-qm",
                "new source"
            ])
            .current_dir(&cwd)
            .status()
            .unwrap()
            .success()
    );
    assert!(guard::run(&job, &AtomicBool::new(false)).is_err());
}

#[test]
fn completed_job_reuse_rejects_changed_runtime_environment() {
    let tmp = tempfile::tempdir().unwrap();
    let cwd = tmp.path().join("source");
    repository(&cwd);
    let invoke = |threads: &str| {
        Command::new(env!("CARGO_BIN_EXE_train_ops"))
            .args(["run", "--job"])
            .arg(tmp.path().join("job"))
            .args(["--timeout", "5", "--memory-gib", "1", "--", "true"])
            .env("OMP_NUM_THREADS", threads)
            .current_dir(&cwd)
            .output()
            .unwrap()
    };
    assert!(invoke("1").status.success());
    assert!(invoke("1").status.success());
    let changed = invoke("2");
    assert!(!changed.status.success());
    assert!(String::from_utf8_lossy(&changed.stderr).contains("preserving existing attempt"));
}
