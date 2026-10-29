use super::*;
use crate::process::test_support::{MockOperations, MockProcess, output, status};

fn clone_spec(base: &str, name: &str) -> Spec {
    Spec::new("tart", ["clone", base, name]).with_timeout(Duration::from_secs(10_800))
}

fn run_spec(name: &str, log: &Path) -> Spec {
    Spec::new("tart", ["run", name, "--no-graphics", "--net-softnet"]).with_log(log.to_path_buf())
}

fn exec_spec(name: &str, args: &[&str], timeout: Duration) -> Spec {
    let mut all = vec!["exec".to_owned(), name.to_owned()];
    all.extend(args.iter().map(|arg| (*arg).to_owned()));
    Spec::new("tart", all).with_timeout(timeout)
}

fn registration_spec(settings: &Settings, name: &str, token: &str) -> Spec {
    let path = format!("repos/{}/actions/runners/generate-jitconfig", settings.repo);
    let args = vec![
        "api".to_owned(),
        "-X".to_owned(),
        "POST".to_owned(),
        path,
        "-f".to_owned(),
        format!("name={name}"),
        "-F".to_owned(),
        "runner_group_id=1".to_owned(),
        "-f".to_owned(),
        "work_folder=_work".to_owned(),
        "-f".to_owned(),
        "labels[]=self-hosted".to_owned(),
        "-f".to_owned(),
        "labels[]=macOS".to_owned(),
        "-f".to_owned(),
        "labels[]=ARM64".to_owned(),
        "-f".to_owned(),
        format!("labels[]={}", settings.label),
    ];
    Spec::new("gh", args)
        .with_env("GH_TOKEN", token)
        .with_timeout(Duration::from_secs(30))
}

fn delete_spec(settings: &Settings, token: &str) -> Spec {
    Spec::new(
        "gh",
        [
            "api",
            "-X",
            "DELETE",
            &format!("repos/{}/actions/runners/42", settings.repo),
        ],
    )
    .with_env("GH_TOKEN", token)
    .with_timeout(Duration::from_secs(30))
    .for_cleanup()
}

#[test]
fn one_job_uses_stdin_and_cleans_vm_and_registration() {
    for cancel in [false, true] {
        let settings = Settings::default();
        let token = "secret-token";
        let name = "vm-one";
        let dir = tempfile::tempdir().unwrap();
        let log = dir.path().join("vm-one.log");
        let mut operations = MockOperations::default();
        if cancel {
            operations.cancel_after_spawns = Some(2);
        }
        operations.push_output(clone_spec(&settings.image, name), output(0, ""));
        operations.push_process(run_spec(name, &log), Box::new(MockProcess::new(status(0))));
        operations.push_output(
            exec_spec(name, &["true"], Duration::from_secs(10)),
            output(0, ""),
        );
        operations.push_output(
            exec_spec(
                name,
                &[
                    "/usr/bin/curl",
                    "-sS",
                    "-m",
                    "3",
                    "-o",
                    "/dev/null",
                    "https://github.com",
                ],
                Duration::from_secs(10),
            ),
            output(0, ""),
        );
        operations.push_output(
            registration_spec(&settings, name, token),
            output(
                0,
                r#"{"runner":{"id":42},"encoded_jit_config":"opaque-secret"}"#,
            ),
        );
        let registration = Registration {
            id: 42,
            encoded: "opaque-secret".to_owned(),
        };
        operations.push_process(
            start_runner_spec(name, &registration, &settings),
            Box::new(MockProcess::with_steps(status(0), &[false, true])),
        );
        operations.push_output(delete_spec(&settings, token), output(0, "{}"));
        operations.push_output(
            Spec::new("tart", ["stop", name])
                .with_timeout(Duration::from_secs(30))
                .for_cleanup(),
            output(0, ""),
        );
        operations.push_output(
            Spec::new("tart", ["delete", name])
                .with_timeout(Duration::from_secs(60))
                .for_cleanup(),
            output(0, ""),
        );
        let result = one_job_named(&settings, &mut operations, name, token, dir.path(), None);
        if cancel {
            assert!(result.unwrap_err().to_string().contains("shutdown"));
        } else {
            assert!(result.unwrap().success());
        }
        assert!(!log.exists());
        operations.assert_done();
        let spec = start_runner_spec(name, &registration, &settings);
        assert!(!spec.args.join(" ").contains("opaque-secret"));
        assert_eq!(spec.stdin.as_deref(), Some(b"opaque-secret\n".as_slice()));
        assert!(start_script(&settings).contains("/Users/admin/.cargo/bin"));
    }
}

/// The commands of one job that is served and cleaned up.
fn push_served_job(operations: &mut MockOperations, settings: &Settings, name: &str, dir: &Path) {
    let token = "token";
    operations.push_output(clone_spec(&settings.image, name), output(0, ""));
    let log = dir.join(format!("{name}.log"));
    operations.push_process(run_spec(name, &log), Box::new(MockProcess::new(status(0))));
    operations.push_output(
        exec_spec(name, &["true"], Duration::from_secs(10)),
        output(0, ""),
    );
    operations.push_output(
        exec_spec(
            name,
            &[
                "/usr/bin/curl",
                "-sS",
                "-m",
                "3",
                "-o",
                "/dev/null",
                "https://github.com",
            ],
            Duration::from_secs(10),
        ),
        output(0, ""),
    );
    operations.push_output(
        registration_spec(settings, name, token),
        output(0, r#"{"runner":{"id":42},"encoded_jit_config":"jit"}"#),
    );
    let registration = Registration {
        id: 42,
        encoded: "jit".to_owned(),
    };
    operations.push_process(
        start_runner_spec(name, &registration, settings),
        Box::new(MockProcess::new(status(0))),
    );
    operations.push_output(delete_spec(settings, token), output(0, "{}"));
    for (command, seconds) in [("stop", 30), ("delete", 60)] {
        operations.push_output(
            Spec::new("tart", [command, name])
                .with_timeout(Duration::from_secs(seconds))
                .for_cleanup(),
            output(0, ""),
        );
    }
}

fn shared(operations: MockOperations) -> std::sync::Arc<std::sync::Mutex<MockOperations>> {
    std::sync::Arc::new(std::sync::Mutex::new(operations))
}

#[test]
fn two_jobs_run_side_by_side_and_the_second_vm_waits_for_the_first_network() {
    let settings = Settings::default();
    let dir = tempfile::tempdir().unwrap();
    let journal = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut first = MockOperations::default();
    push_served_job(&mut first, &settings, "vm-a", dir.path());
    first.journal = Some(journal.clone());
    // A slow network check would let a second VM start beside it, and a slow
    // runner start keeps the first job busy while the second VM begins.
    let delay = Duration::from_millis(300);
    first.slow = vec![
        ("https://github.com".to_owned(), delay),
        ("-i".to_owned(), delay),
    ];
    let mut second = MockOperations::default();
    push_served_job(&mut second, &settings, "vm-b", dir.path());
    second.journal = Some(journal.clone());
    let (first, second) = (shared(first), shared(second));
    let mut operations = MockOperations::default();
    operations.workers = [first.clone(), second.clone()].into();

    let names = ["vm-a".to_owned(), "vm-b".to_owned()];
    assert!(run_jobs(
        &settings,
        &mut operations,
        &names,
        "token",
        dir.path()
    ));
    first.lock().unwrap().assert_done();
    second.lock().unwrap().assert_done();
    operations.assert_done();

    let journal = journal.lock().unwrap();
    let at = |line: &str| {
        journal
            .iter()
            .position(|entry| entry.starts_with(line))
            .unwrap_or_else(|| panic!("{line} never ran: {journal:#?}"))
    };
    let second_clone = at("tart clone baylee-runner vm-b");
    assert!(
        at("tart exec vm-a /usr/bin/curl") < second_clone,
        "{journal:#?}"
    );
    assert!(second_clone < at("tart stop vm-a"), "{journal:#?}");
}

#[test]
fn a_vm_that_fails_before_the_network_still_lets_the_next_one_start() {
    let settings = Settings::default();
    let dir = tempfile::tempdir().unwrap();
    let mut first = MockOperations::default();
    first.push_output(clone_spec(&settings.image, "vm-a"), output(1, ""));
    let mut second = MockOperations::default();
    push_served_job(&mut second, &settings, "vm-b", dir.path());
    let (first, second) = (shared(first), shared(second));
    let mut operations = MockOperations::default();
    operations.workers = [first.clone(), second.clone()].into();

    let names = ["vm-a".to_owned(), "vm-b".to_owned()];
    assert!(run_jobs(
        &settings,
        &mut operations,
        &names,
        "token",
        dir.path()
    ));
    first.lock().unwrap().assert_done();
    second.lock().unwrap().assert_done();
}

#[test]
fn a_single_job_runs_on_this_thread() {
    let settings = Settings::default();
    let dir = tempfile::tempdir().unwrap();
    let mut operations = MockOperations::default();
    push_served_job(&mut operations, &settings, "vm-one", dir.path());
    let unused = shared(MockOperations::default());
    operations.workers = [unused].into();
    let names = ["vm-one".to_owned()];
    assert!(run_jobs(
        &settings,
        &mut operations,
        &names,
        "token",
        dir.path()
    ));
    operations.assert_done();
    assert_eq!(
        operations.workers.len(),
        1,
        "a single job asked for a worker"
    );
}

#[test]
fn a_network_failure_still_deletes_the_vm_without_registering() {
    let settings = Settings::default();
    let name = "vm-offline";
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("vm-offline.log");
    let mut operations = MockOperations::default();
    operations.push_output(clone_spec(&settings.image, name), output(0, ""));
    operations.push_process(run_spec(name, &log), Box::new(MockProcess::new(status(0))));
    operations.push_output(
        exec_spec(name, &["true"], Duration::from_secs(10)),
        output(0, ""),
    );
    for _ in 0..30 {
        operations.push_output(
            exec_spec(
                name,
                &[
                    "/usr/bin/curl",
                    "-sS",
                    "-m",
                    "3",
                    "-o",
                    "/dev/null",
                    "https://github.com",
                ],
                Duration::from_secs(10),
            ),
            output(1, ""),
        );
    }
    operations.push_output(
        Spec::new("tart", ["stop", name])
            .with_timeout(Duration::from_secs(30))
            .for_cleanup(),
        output(0, ""),
    );
    operations.push_output(
        Spec::new("tart", ["delete", name])
            .with_timeout(Duration::from_secs(60))
            .for_cleanup(),
        output(0, ""),
    );
    let error = one_job_named(&settings, &mut operations, name, "token", dir.path(), None)
        .expect_err("offline VM must not register");
    assert!(
        error.to_string().contains("no way to github.com"),
        "{error}"
    );
    operations.assert_done();
}

#[test]
fn an_idle_registration_is_stopped_and_removed() {
    let settings = Settings::for_tests();
    let token = "secret-token";
    let name = "vm-idle";
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("vm-idle.log");
    let mut operations = MockOperations::default();
    operations.push_output(clone_spec(&settings.image, name), output(0, ""));
    operations.push_process(run_spec(name, &log), Box::new(MockProcess::new(status(0))));
    operations.push_output(
        exec_spec(name, &["true"], Duration::from_secs(10)),
        output(0, ""),
    );
    operations.push_output(
        exec_spec(
            name,
            &[
                "/usr/bin/curl",
                "-sS",
                "-m",
                "3",
                "-o",
                "/dev/null",
                "https://github.com",
            ],
            Duration::from_secs(10),
        ),
        output(0, ""),
    );
    operations.push_output(
        registration_spec(&settings, name, token),
        output(
            0,
            r#"{"runner":{"id":42},"encoded_jit_config":"opaque-secret"}"#,
        ),
    );
    let registration = Registration {
        id: 42,
        encoded: "opaque-secret".to_owned(),
    };
    operations.push_process(
        start_runner_spec(name, &registration, &settings),
        Box::new(MockProcess::new(status(0))),
    );
    operations.push_output(
        Spec::new(
            "gh",
            [
                "api",
                &format!("repos/{}/actions/runners/42", settings.repo),
            ],
        )
        .with_env("GH_TOKEN", token)
        .with_timeout(Duration::from_secs(30)),
        output(0, r#"{"busy":false}"#),
    );
    operations.push_output(
        Spec::new("tart", ["stop", name])
            .with_timeout(Duration::from_secs(30))
            .for_cleanup(),
        output(0, ""),
    );
    operations.push_output(delete_spec(&settings, token), output(0, "{}"));
    operations.push_output(
        Spec::new("tart", ["stop", name])
            .with_timeout(Duration::from_secs(30))
            .for_cleanup(),
        output(0, ""),
    );
    operations.push_output(
        Spec::new("tart", ["delete", name])
            .with_timeout(Duration::from_secs(60))
            .for_cleanup(),
        output(0, ""),
    );
    let result = one_job_named(&settings, &mut operations, name, token, dir.path(), None).unwrap();
    assert!(result.success());
    operations.assert_done();
}

#[test]
#[allow(clippy::too_many_lines)] // one complete image-provisioning checklist
fn prepare_builds_the_image_without_a_start_runner_file() {
    let settings = Settings::default();
    let name = settings.image.as_str();
    let mut operations = MockOperations::default();
    operations.push_output(
        Spec::new("tart", ["list", "--quiet", "--source", "local"]),
        output(0, ""),
    );
    operations.push_output(clone_spec(&settings.base, name), output(0, ""));
    let cpus = settings.vm_cpus.to_string();
    let memory = settings.vm_memory_mb.to_string();
    let disk = settings.vm_disk_gb.to_string();
    operations.push_output(
        Spec::new(
            "tart",
            [
                "set",
                name,
                "--cpu",
                &cpus,
                "--memory",
                &memory,
                "--disk-size",
                &disk,
            ],
        ),
        output(0, ""),
    );
    operations.push_process(
        Spec::new("tart", ["run", name, "--no-graphics", "--net-softnet"]),
        Box::new(MockProcess::new(status(0))),
    );
    operations.push_output(
        exec_spec(name, &["true"], Duration::from_secs(10)),
        output(0, ""),
    );
    operations.push_output(
        exec_spec(
            name,
            &[
                "/usr/bin/curl",
                "--proto",
                "=https",
                "--tlsv1.2",
                "-fsSLo",
                "/tmp/rustup-init.sh",
                "https://sh.rustup.rs",
            ],
            Duration::from_secs(600),
        ),
        output(0, ""),
    );
    operations.push_output(
        exec_spec(
            name,
            &[
                "/bin/sh",
                "/tmp/rustup-init.sh",
                "-y",
                "--profile",
                "minimal",
                "--default-toolchain",
                "stable",
                "--target",
                "x86_64-apple-darwin",
            ],
            Duration::from_secs(600),
        ),
        output(0, ""),
    );
    operations.push_output(
        exec_spec(
            name,
            &["/bin/mkdir", "-p", RUNNER_DIR],
            Duration::from_secs(600),
        ),
        output(0, ""),
    );
    let archive = format!(
        "https://github.com/actions/runner/releases/download/v{}/actions-runner-osx-arm64-{}.tar.gz",
        settings.runner_version, settings.runner_version
    );
    operations.push_output(
        exec_spec(
            name,
            &[
                "/usr/bin/curl",
                "-fsSLo",
                "/Users/admin/runner.tar.gz",
                &archive,
            ],
            Duration::from_secs(600),
        ),
        output(0, ""),
    );
    operations.push_output(
        exec_spec(
            name,
            &["/usr/bin/shasum", "-a", "256", "/Users/admin/runner.tar.gz"],
            Duration::from_secs(60),
        ),
        output(
            0,
            &format!("{}  /Users/admin/runner.tar.gz\n", settings.runner_sha256),
        ),
    );
    operations.push_output(
        exec_spec(
            name,
            &[
                "/usr/bin/tar",
                "-xzf",
                "/Users/admin/runner.tar.gz",
                "-C",
                RUNNER_DIR,
            ],
            Duration::from_secs(600),
        ),
        output(0, ""),
    );
    operations.push_output(
        exec_spec(
            name,
            &[
                "/bin/rm",
                "/Users/admin/runner.tar.gz",
                "/tmp/rustup-init.sh",
            ],
            Duration::from_secs(600),
        ),
        output(0, ""),
    );
    operations.push_output(
        exec_spec(
            name,
            &["/usr/bin/arch", "-x86_64", "/usr/bin/true"],
            Duration::from_secs(600),
        ),
        output(0, ""),
    );
    operations.push_output(
        Spec::new("tart", ["stop", name])
            .with_timeout(Duration::from_secs(30))
            .for_cleanup(),
        output(0, ""),
    );
    prepare(&settings, &mut operations).unwrap();
    operations.assert_done();
}

/// The job guard runs as the job-started hook; a non-zero exit stops the
/// job before its first step. Run here with the real shell.
#[cfg(unix)]
#[test]
fn the_job_guard_admits_only_main_pushes_and_dispatches_of_this_repository() {
    let settings = Settings::default();
    let dir = tempfile::tempdir().unwrap();
    let guard = dir.path().join("job-guard.sh");
    std::fs::write(&guard, job_guard(&settings.repo)).unwrap();
    let admits = |repository: &str, event: &str, git_ref: &str| {
        std::process::Command::new("/bin/sh")
            .arg(&guard)
            .env_clear()
            .env("GITHUB_REPOSITORY", repository)
            .env("GITHUB_EVENT_NAME", event)
            .env("GITHUB_REF", git_ref)
            .output()
            .unwrap()
            .status
            .success()
    };
    let repo = settings.repo.as_str();
    assert!(admits(repo, "push", "refs/heads/main"));
    assert!(admits(repo, "workflow_dispatch", "refs/heads/c41/topic"));
    assert!(!admits(repo, "pull_request", "refs/pull/7/merge"));
    assert!(!admits(repo, "push", "refs/heads/c41/topic"));
    assert!(!admits(repo, "push", "refs/tags/v1.0.0"));
    assert!(!admits("someone/baylee", "push", "refs/heads/main"));
    assert!(!admits(repo, "pull_request_target", "refs/heads/main"));

    let script = start_script(&settings);
    assert!(script.contains(&format!("export ACTIONS_RUNNER_HOOK_JOB_STARTED={GUARD}")));
    assert!(script.contains(&job_guard(repo)));
    // The hook is in place before the runner starts.
    assert!(script.find(GUARD).unwrap() < script.find("exec ./run.sh").unwrap());
}

/// A runner killed outright (SIGKILL after launchd's 180 s) leaves its VM
/// and its registration; the next `serve` removes both before it serves,
/// and nothing that may still be in use.
#[test]
fn a_start_sweeps_what_a_killed_runner_left_and_nothing_else() {
    let settings = Settings::default();
    let token = "token";
    let mut operations = MockOperations::default();
    let vms = r#"[
        {"Name": "baylee-runner", "State": "stopped"},
        {"Name": "baylee-vm-1793286183-1", "State": "stopped"},
        {"Name": "baylee-vm-1793286183-2", "State": "running"},
        {"Name": "exp-a1", "State": "stopped"}
    ]"#;
    operations.push_output(
        Spec::new("tart", ["list", "--source", "local", "--format", "json"])
            .with_timeout(Duration::from_secs(30)),
        output(0, vms),
    );
    operations.push_output(
        Spec::new("tart", ["delete", "baylee-vm-1793286183-1"])
            .with_timeout(Duration::from_secs(60))
            .for_cleanup(),
        output(0, ""),
    );
    let runners = r#"{"runners": [
        {"id": 41, "name": "baylee-vm-1793286183-1", "status": "offline"},
        {"id": 42, "name": "baylee-vm-1793286183-2", "status": "online"},
        {"id": 43, "name": "someone-else", "status": "offline"}
    ]}"#;
    operations.push_output(
        Spec::new(
            "gh",
            [
                "api",
                &format!("repos/{}/actions/runners?per_page=100", settings.repo),
            ],
        )
        .with_env("GH_TOKEN", token)
        .with_timeout(Duration::from_secs(30)),
        output(0, runners),
    );
    operations.push_output(
        Spec::new(
            "gh",
            [
                "api",
                "-X",
                "DELETE",
                &format!("repos/{}/actions/runners/41", settings.repo),
            ],
        )
        .with_env("GH_TOKEN", token)
        .with_timeout(Duration::from_secs(30))
        .for_cleanup(),
        output(0, ""),
    );
    sweep(&settings, &mut operations, token);
    operations.assert_done();
}
