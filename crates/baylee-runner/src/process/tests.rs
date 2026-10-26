use super::test_support::{MockOperations, MockProcess, output, status};
use super::*;
use std::time::Instant;

#[test]
fn debug_never_prints_environment_values_or_stdin() {
    let spec = Spec::new("gh", ["api"])
        .with_env("GH_TOKEN", "secret-value")
        .with_stdin(b"registration-secret".to_vec());
    let shown = format!("{spec:?}");
    assert!(!shown.contains("secret-value"));
    assert!(!shown.contains("registration-secret"));
    assert!(shown.contains("GH_TOKEN"));
}

#[cfg(unix)]
#[test]
fn output_timeout_kills_the_child() {
    let mut real = RealOperations;
    let error = real
        .output(Spec::new("/bin/sh", ["-c", "sleep 5"]).with_timeout(Duration::from_millis(30)))
        .expect_err("the child must be stopped");
    assert!(error.to_string().contains("timed out"), "{error}");
}

#[test]
fn a_mock_child_can_finish_on_a_later_poll() {
    let mut process = MockProcess::with_steps(status(0), &[false, true]);
    assert_eq!(process.try_wait().unwrap(), None);
    assert_eq!(process.try_wait().unwrap(), Some(status(0)));
}

#[test]
fn the_mock_boundary_checks_every_argument() {
    let spec = Spec::new("gh", ["api", "repos/example"]);
    let mut mock = MockOperations::default();
    mock.push_output(spec.clone(), output(0, "{}"));
    let got = mock.output(spec).unwrap();
    assert_eq!(got.stdout, b"{}");
    mock.assert_done();
}

#[cfg(unix)]
#[test]
fn captured_output_deadline_includes_term_resistant_descendants() {
    let started = Instant::now();
    let result = RealOperations.output(
        Spec::new("/bin/sh", ["-c", "trap '' TERM; sleep 3; printf done"])
            .with_timeout(Duration::from_millis(100)),
    );
    assert!(result.is_err());
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "timeout waited for descendants"
    );
}

#[cfg(unix)]
#[test]
fn streaming_deadline_includes_inherited_output_pipes() {
    let started = Instant::now();
    let spec = Spec::new("/bin/sh", ["-c", "sleep 3 & exit 0"])
        .with_prefix("test:".to_owned())
        .with_timeout(Duration::from_millis(100));
    let result = RealOperations
        .spawn(spec)
        .unwrap()
        .wait(Duration::from_millis(100));
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "stream join ignored the deadline"
    );
    assert!(result.is_err(), "descendant kept the output pipe open");
}

#[cfg(unix)]
#[test]
fn stdin_delivery_is_inside_the_command_deadline() {
    for capture in [false, true] {
        let started = Instant::now();
        let spec = Spec::new("/bin/sh", ["-c", "sleep 3"])
            .with_stdin(vec![b'x'; 1024 * 1024])
            .with_timeout(Duration::from_millis(100));
        if capture {
            assert!(RealOperations.output(spec).is_err());
        } else {
            assert!(
                RealOperations
                    .spawn(spec)
                    .and_then(|mut child| child.wait(Duration::from_millis(100)))
                    .is_err()
            );
        }
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "stdin blocked before supervision"
        );
    }
}

#[cfg(unix)]
#[test]
fn successful_capture_drains_both_streams_while_writing_stdin() {
    let input = vec![b'x'; 128 * 1024];
    let output = RealOperations
        .output(
            Spec::new("/bin/sh", ["-c", "cat; printf diagnostic >&2"])
                .with_stdin(input.clone())
                .with_timeout(Duration::from_secs(5)),
        )
        .unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, input);
    assert_eq!(output.stderr, b"diagnostic");
}

#[cfg(unix)]
#[test]
fn shutdown_subprocess_helper() {
    let Some(ready) = std::env::var_os("BAYLEE_RUNNER_TEST_READY") else {
        return;
    };
    crate::shutdown::install().unwrap();
    let mut operations = RealOperations;
    let mut process = operations
        .spawn(Spec::new("/bin/sh", ["-c", "sleep 10"]).with_prefix("shutdown:".to_owned()))
        .unwrap();
    std::fs::write(&ready, b"ready").unwrap();
    assert!(process.wait(Duration::from_secs(10)).is_err());
    drop(process);
    assert!(operations.cancelled());
    // Cleanup still works after the signal, including captured output.
    let output = operations
        .output(Spec::new("/bin/echo", ["cleaned"]).for_cleanup())
        .unwrap();
    assert_eq!(output.stdout, b"cleaned\n");
}

#[cfg(unix)]
#[test]
fn termination_unwinds_processes_and_allows_cleanup_commands() {
    let dir = tempfile::tempdir().unwrap();
    let ready = dir.path().join("ready");
    let exe = std::env::current_exe().unwrap();
    let mut child = RealOperations
        .spawn(
            Spec::new(
                exe.to_str().unwrap(),
                [
                    "--exact",
                    "process::tests::shutdown_subprocess_helper",
                    "--nocapture",
                ],
            )
            .with_env("BAYLEE_RUNNER_TEST_READY", ready.to_str().unwrap())
            .with_prefix("child:".to_owned())
            .with_timeout(Duration::from_secs(10)),
        )
        .unwrap();
    let started = Instant::now();
    while !ready.exists() && started.elapsed() < Duration::from_secs(5) {
        thread::sleep(Duration::from_millis(10));
    }
    assert!(ready.exists(), "child did not initialize");
    child.terminate().unwrap();
    assert!(child.wait(Duration::from_secs(2)).unwrap().success());
}
