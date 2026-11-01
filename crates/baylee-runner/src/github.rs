//! GitHub's queue and just-in-time runner registration.

use crate::process::{Operations, Spec, require_success};
use anyhow::{Context, Result, bail};
use serde_json::Value;
use std::time::Duration;

/// The JIT registration returned for exactly one VM and one job.
pub struct Registration {
    /// Numeric GitHub runner id, safe to log.
    pub id: u64,
    /// Opaque JIT configuration, passed only to the guest over stdin.
    pub encoded: String,
}

/// GitHub calls made through `gh`, with the token only in each child's environment.
pub struct GitHub<'a> {
    operations: &'a mut dyn Operations,
    repo: &'a str,
    token: &'a str,
}

impl<'a> GitHub<'a> {
    /// Creates the narrow process wrapper.
    pub fn new(operations: &'a mut dyn Operations, repo: &'a str, token: &'a str) -> Self {
        Self {
            operations,
            repo,
            token,
        }
    }

    fn spec<I, S>(&self, args: I) -> Spec
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Spec::new("gh", args)
            .with_env("GH_TOKEN", self.token)
            .with_timeout(Duration::from_secs(30))
    }

    fn api(&mut self, args: &[&str]) -> Result<serde_json::Value> {
        let output = self.operations.output(self.spec(args.iter().copied()))?;
        require_success(&output, "gh api")?;
        serde_json::from_slice(&output.stdout).context("decoding GitHub JSON")
    }

    /// Counts queued jobs labelled `label` in all unfinished runs. API errors
    /// deliberately look like an empty queue, as in the shell runner.
    pub fn queued(&mut self, label: &str) -> usize {
        let mut runs = Vec::new();
        for status in ["in_progress", "queued"] {
            let path = format!(
                "repos/{}/actions/runs?status={status}&per_page=20",
                self.repo
            );
            let Ok(answer) = self.api(&["api", &path]) else {
                return 0;
            };
            let Ok(ids) = run_ids(&answer, self.repo) else {
                return 0;
            };
            runs.extend(ids);
        }
        let mut count = 0;
        for run in runs {
            let path = format!(
                "repos/{}/actions/runs/{run}/jobs?filter=latest&per_page=100",
                self.repo
            );
            let Ok(answer) = self.api(&["api", &path]) else {
                return 0;
            };
            let Ok(jobs) = queued_jobs(&answer, label) else {
                return 0;
            };
            count += jobs;
        }
        count
    }

    /// Registers one labelled JIT runner. The opaque reply is never logged.
    pub fn register(&mut self, name: &str, label: &str) -> Result<Registration> {
        let name_arg = format!("name={name}");
        let group_arg = "runner_group_id=1";
        let work_arg = "work_folder=_work";
        let labels = ["self-hosted", "macOS", "ARM64", label];
        let path = format!("repos/{}/actions/runners/generate-jitconfig", self.repo);
        let mut args = vec![
            "api".to_owned(),
            "-X".to_owned(),
            "POST".to_owned(),
            path,
            "-f".to_owned(),
            name_arg,
            "-F".to_owned(),
            group_arg.to_owned(),
            "-f".to_owned(),
            work_arg.to_owned(),
        ];
        for label in labels {
            args.push("-f".to_owned());
            args.push(format!("labels[]={label}"));
        }
        let output = self.operations.output(self.spec(args))?;
        if !output.status.success() {
            bail!("GitHub JIT registration failed");
        }
        let answer: Value =
            serde_json::from_slice(&output.stdout).context("decoding JIT registration")?;
        let id = answer
            .pointer("/runner/id")
            .and_then(Value::as_u64)
            .context("JIT response has no runner id")?;
        let encoded = answer
            .get("encoded_jit_config")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty() && !value.chars().any(char::is_whitespace));
        let Some(encoded) = encoded else {
            self.remove(id);
            bail!("JIT response has no valid encoded configuration");
        };
        Ok(Registration {
            id,
            encoded: encoded.to_owned(),
        })
    }

    /// Whether a registered runner currently has a job. Unknown status must not
    /// abort a potentially busy job; the absolute VM deadline still applies.
    pub fn busy(&mut self, id: u64) -> Option<bool> {
        let path = format!("repos/{}/actions/runners/{id}", self.repo);
        self.api(&["api", &path])
            .ok()
            .and_then(|answer| answer.get("busy").and_then(Value::as_bool))
    }

    /// This repository's offline runners whose name starts with `prefix`:
    /// registrations a runner killed outright never removed. An API error is
    /// an empty list; the next start tries again.
    pub fn offline_runners(&mut self, prefix: &str) -> Vec<(u64, String)> {
        let path = format!("repos/{}/actions/runners?per_page=100", self.repo);
        let Ok(answer) = self.api(&["api", &path]) else {
            return Vec::new();
        };
        answer
            .get("runners")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|runner| runner.get("status").and_then(Value::as_str) == Some("offline"))
            .filter_map(|runner| {
                let id = runner.get("id").and_then(Value::as_u64)?;
                let name = runner.get("name").and_then(Value::as_str)?;
                name.starts_with(prefix).then(|| (id, name.to_owned()))
            })
            .collect()
    }

    /// Deletes a JIT registration, best effort after job completion.
    pub fn remove(&mut self, id: u64) {
        let path = format!("repos/{}/actions/runners/{id}", self.repo);
        let spec = self.spec(["api", "-X", "DELETE", &path]).for_cleanup();
        let _ = self.operations.output(spec);
    }
}

/// The runs a VM may serve: this repository's pushes to main and its
/// dispatches, which only someone with write access can start. A pull
/// request, a fork's above all, may name the label in its own YAML; it is
/// left queued here, and the job guard in each VM refuses it as well.
fn run_ids(answer: &Value, repo: &str) -> Result<Vec<u64>> {
    let runs = answer
        .get("workflow_runs")
        .and_then(Value::as_array)
        .context("GitHub response has no workflow_runs array")?;
    let mut ids = Vec::new();
    for run in runs {
        let id = run
            .get("id")
            .and_then(Value::as_u64)
            .context("run has no id")?;
        let field = |pointer: &str| run.pointer(pointer).and_then(Value::as_str);
        let trusted = field("/head_repository/full_name") == Some(repo)
            && match field("/event") {
                Some("push") => field("/head_branch") == Some("main"),
                Some("workflow_dispatch") => true,
                _ => false,
            };
        if trusted {
            ids.push(id);
        }
    }
    Ok(ids)
}

fn queued_jobs(answer: &Value, label: &str) -> Result<usize> {
    let jobs = answer
        .get("jobs")
        .and_then(Value::as_array)
        .context("GitHub response has no jobs array")?;
    Ok(jobs
        .iter()
        .filter(|job| {
            job.get("status").and_then(Value::as_str) == Some("queued")
                && job
                    .get("labels")
                    .and_then(Value::as_array)
                    .is_some_and(|labels| labels.iter().any(|name| name.as_str() == Some(label)))
        })
        .count())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process::test_support::{MockOperations, output};
    use serde_json::json;

    #[test]
    fn queue_counts_only_queued_jobs_with_the_runner_label() {
        let answer = json!({"jobs": [
            {"status": "queued", "labels": ["self-hosted", "baylee-vm"]},
            {"status": "queued", "labels": ["macos-latest"]},
            {"status": "in_progress", "labels": ["baylee-vm"]},
            {"status": "queued", "labels": ["self-hosted", "macOS", "baylee-vm"]}
        ]});
        assert_eq!(queued_jobs(&answer, "baylee-vm").unwrap(), 2);
    }

    #[test]
    fn run_ids_requires_numeric_ids() {
        let repo = "AceVik/baylee";
        let run = |id: Value| {
            json!({"id": id, "event": "push", "head_branch": "main",
                   "head_repository": {"full_name": repo}})
        };
        assert_eq!(
            run_ids(
                &json!({"workflow_runs": [run(json!(7)), run(json!(11))]}),
                repo
            )
            .unwrap(),
            [7, 11]
        );
        assert!(run_ids(&json!({"workflow_runs": [run(json!("7"))]}), repo).is_err());
    }

    #[test]
    fn only_main_pushes_and_dispatches_of_this_repository_are_served() {
        let repo = "AceVik/baylee";
        let run = |id: u64, event: &str, branch: &str, head: &str| {
            json!({"id": id, "event": event, "head_branch": branch,
                   "head_repository": {"full_name": head}})
        };
        let answer = json!({"workflow_runs": [
            run(1, "push", "main", repo),
            run(2, "workflow_dispatch", "c41/topic", repo),
            run(3, "pull_request", "main", "someone/baylee"),
            run(4, "pull_request", "c41/topic", repo),
            run(5, "push", "c41/topic", repo),
            run(6, "push", "main", "someone/baylee"),
            {"id": 7, "event": "push", "head_branch": "main"},
        ]});
        assert_eq!(run_ids(&answer, repo).unwrap(), [1, 2]);
    }

    #[test]
    fn api_failure_is_an_empty_queue_not_a_job() {
        let mut operations = MockOperations::default();
        let repo = "AceVik/baylee";
        let token = "secret-token";
        operations.push_output(
            Spec::new(
                "gh",
                [
                    "api",
                    "repos/AceVik/baylee/actions/runs?status=in_progress&per_page=20",
                ],
            )
            .with_env("GH_TOKEN", token)
            .with_timeout(Duration::from_secs(30)),
            output(1, ""),
        );
        let count = GitHub::new(&mut operations, repo, token).queued("baylee-vm");
        assert_eq!(count, 0);
        operations.assert_done();
    }

    #[test]
    fn busy_keeps_unknown_distinct_from_idle() {
        for (reply, expected) in [
            (output(0, r#"{"busy":true}"#), Some(true)),
            (output(0, r#"{"busy":false}"#), Some(false)),
            (output(1, ""), None),
            (output(0, "{}"), None),
        ] {
            let mut operations = MockOperations::default();
            operations.push_output(
                Spec::new("gh", ["api", "repos/example/repo/actions/runners/42"])
                    .with_env("GH_TOKEN", "fixture")
                    .with_timeout(Duration::from_secs(30)),
                reply,
            );
            assert_eq!(
                GitHub::new(&mut operations, "example/repo", "fixture").busy(42),
                expected
            );
            operations.assert_done();
        }
    }

    #[test]
    fn registration_returns_id_and_opaque_config_without_logging_it() {
        for encoded in ["opaque-secret", "", "line\nbreak"] {
            let mut operations = MockOperations::default();
            let repo = "AceVik/baylee";
            let token = "secret-token";
            let expected = Spec::new(
                "gh",
                [
                    "api",
                    "-X",
                    "POST",
                    "repos/AceVik/baylee/actions/runners/generate-jitconfig",
                    "-f",
                    "name=vm-one",
                    "-F",
                    "runner_group_id=1",
                    "-f",
                    "work_folder=_work",
                    "-f",
                    "labels[]=self-hosted",
                    "-f",
                    "labels[]=macOS",
                    "-f",
                    "labels[]=ARM64",
                    "-f",
                    "labels[]=baylee-vm",
                ],
            )
            .with_env("GH_TOKEN", token)
            .with_timeout(Duration::from_secs(30));
            operations.push_output(
                expected,
                output(
                    0,
                    &json!({"runner": {"id": 42}, "encoded_jit_config": encoded}).to_string(),
                ),
            );
            let valid = encoded == "opaque-secret";
            if !valid {
                operations.push_output(
                    Spec::new(
                        "gh",
                        [
                            "api",
                            "-X",
                            "DELETE",
                            "repos/AceVik/baylee/actions/runners/42",
                        ],
                    )
                    .with_env("GH_TOKEN", token)
                    .with_timeout(Duration::from_secs(30))
                    .for_cleanup(),
                    output(0, ""),
                );
            }
            let result = GitHub::new(&mut operations, repo, token).register("vm-one", "baylee-vm");
            if valid {
                let registration = result.unwrap();
                assert_eq!(registration.id, 42);
                assert_eq!(registration.encoded, "opaque-secret");
            } else {
                assert!(result.is_err());
            }
            operations.assert_done();
        }
    }
}
