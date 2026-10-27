use super::test_support::{MockOperations, MockProcess, output, status};
use super::*;

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
