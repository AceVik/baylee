//! What a pass asks GitHub, in order, and how little it takes to mean "no job".

use super::*;
use crate::process::test_support::{MockOperations, output};
use serde_json::json;

const REPO: &str = "AceVik/baylee";

fn asked(path: &str) -> Spec {
    Spec::new("gh", ["api", path])
        .with_env("GH_TOKEN", "fixture")
        .with_timeout(Duration::from_secs(30))
}

fn runs(status: &str) -> Spec {
    asked(&format!(
        "repos/{REPO}/actions/runs?status={status}&per_page=20"
    ))
}

fn jobs(run: u64) -> Spec {
    asked(&format!(
        "repos/{REPO}/actions/runs/{run}/jobs?filter=latest&per_page=100"
    ))
}

fn dispatched(ids: &[u64]) -> String {
    let runs: Vec<_> = ids
        .iter()
        .map(|id| {
            json!({"id": id, "event": "workflow_dispatch",
                   "head_repository": {"full_name": REPO}})
        })
        .collect();
    json!({"workflow_runs": runs}).to_string()
}

fn waiting(jobs: usize) -> String {
    let job = json!({"status": "queued", "labels": ["baylee-vm"]});
    json!({"jobs": vec![job; jobs]}).to_string()
}

/// Asks for the queue with these answers, in this order.
fn queued(answers: Vec<(Spec, String)>) -> usize {
    let mut operations = MockOperations::default();
    for (spec, answer) in answers {
        operations.push_output(spec, output(0, &answer));
    }
    let count = GitHub::new(&mut operations, REPO, "fixture").queued("baylee-vm");
    operations.assert_done();
    count
}

#[test]
fn the_queue_adds_up_the_jobs_of_running_and_waiting_runs() {
    let count = queued(vec![
        (runs("in_progress"), dispatched(&[3])),
        (runs("queued"), dispatched(&[5, 8])),
        (jobs(3), waiting(1)),
        (jobs(5), waiting(0)),
        (jobs(8), waiting(2)),
    ]);
    assert_eq!(count, 3);
}

#[test]
fn an_answer_that_cannot_be_read_starts_no_runner() {
    // Half a count is no count: the first run's job is not served either.
    let count = queued(vec![
        (runs("in_progress"), dispatched(&[3, 5])),
        (runs("queued"), dispatched(&[])),
        (jobs(3), waiting(1)),
        (jobs(5), "not json".into()),
    ]);
    assert_eq!(count, 0);
    let count = queued(vec![
        (runs("in_progress"), dispatched(&[3])),
        (runs("queued"), dispatched(&[])),
        (jobs(3), "{}".into()),
    ]);
    assert_eq!(count, 0);
    let count = queued(vec![
        (runs("in_progress"), dispatched(&[3])),
        (runs("queued"), "{}".into()),
    ]);
    assert_eq!(count, 0);
}

#[test]
fn only_offline_runners_of_our_own_name_are_swept() {
    let answer = json!({"runners": [
        {"id": 1, "name": "baylee-vm-1", "status": "offline"},
        {"id": 2, "name": "baylee-vm-2", "status": "online"},
        {"id": 3, "name": "somebody-elses", "status": "offline"},
        {"id": 4, "name": "not-baylee-vm-4", "status": "offline"},
        {"id": "5", "name": "baylee-vm-5", "status": "offline"},
        {"id": 6, "status": "offline"},
        {"id": 7, "name": "baylee-vm-7"},
    ]});
    let mut operations = MockOperations::default();
    operations.push_output(
        asked(&format!("repos/{REPO}/actions/runners?per_page=100")),
        output(0, &answer.to_string()),
    );
    let offline = GitHub::new(&mut operations, REPO, "fixture").offline_runners("baylee-vm-");
    operations.assert_done();
    assert_eq!(offline, [(1, "baylee-vm-1".to_owned())]);
}

#[test]
fn a_list_of_runners_that_cannot_be_read_sweeps_nothing() {
    for reply in [
        output(1, r#"{"runners":[]}"#),
        output(0, "{}"),
        output(0, ""),
    ] {
        let mut operations = MockOperations::default();
        operations.push_output(
            asked(&format!("repos/{REPO}/actions/runners?per_page=100")),
            reply,
        );
        let offline = GitHub::new(&mut operations, REPO, "fixture").offline_runners("");
        assert!(offline.is_empty());
    }
}

#[test]
fn a_runner_is_removed_by_its_number_as_cleanup() {
    let mut operations = MockOperations::default();
    let path = format!("repos/{REPO}/actions/runners/42");
    operations.push_output(
        Spec::new("gh", ["api", "-X", "DELETE", &path])
            .with_env("GH_TOKEN", "fixture")
            .with_timeout(Duration::from_secs(30))
            .for_cleanup(),
        output(1, ""),
    );
    GitHub::new(&mut operations, REPO, "fixture").remove(42);
    operations.assert_done();
}
