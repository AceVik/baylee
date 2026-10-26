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
        let result = one_job_named(&settings, &mut operations, name, token, dir.path());
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
        assert!(START_SCRIPT.contains("/Users/admin/.cargo/bin"));
    }
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
    let error = one_job_named(&settings, &mut operations, name, "token", dir.path())
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
    let result = one_job_named(&settings, &mut operations, name, token, dir.path()).unwrap();
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
