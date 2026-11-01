//! Fail-closed CI run policy, with an injectable JSON API for offline tests.
use anyhow::{Context, Result, bail, ensure};
use serde_json::Value;
use std::{env, process::Command};

pub(super) const TARGETS: [(&str, &str); 6] = [
    ("x86_64-unknown-linux-gnu", "tar.gz"),
    ("aarch64-unknown-linux-gnu", "tar.gz"),
    ("x86_64-pc-windows-msvc", "zip"),
    ("aarch64-pc-windows-msvc", "zip"),
    ("aarch64-apple-darwin", "zip"),
    ("x86_64-apple-darwin", "zip"),
];
pub(super) const REQUIRED_JOBS: [&str; 11] = [
    "fmt",
    "clippy",
    "test",
    "test-release",
    "shipped-features",
    "validate",
    "web-feedback",
    "deny",
    "audit",
    "bench",
    "msrv",
];

pub(super) struct Identity {
    pub repository: String,
    pub commit: String,
}
impl Identity {
    pub fn environment() -> Result<Self> {
        Ok(Self {
            repository: env::var("GITHUB_REPOSITORY")?,
            commit: env::var("GITHUB_SHA")?,
        })
    }
}

pub(super) fn gh(path: &str) -> Result<Value> {
    let output = Command::new("gh")
        .args(["api", path])
        .output()
        .context("running gh api")?;
    ensure!(
        output.status.success(),
        "gh api {path}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).context("invalid GitHub API JSON")
}

pub(super) fn trusted_run(run: &Value, identity: &Identity) -> bool {
    run["head_sha"].as_str() == Some(&identity.commit)
        && run["head_branch"].as_str() == Some("main")
        && run["event"].as_str() == Some("push")
        && run["status"].as_str() == Some("completed")
        && run["conclusion"].as_str() == Some("success")
        && run["path"].as_str() == Some(".github/workflows/ci.yml")
        && run["repository"]["full_name"].as_str() == Some(&identity.repository)
        && run["head_repository"]["full_name"].as_str() == Some(&identity.repository)
}

pub(super) fn pages(
    api: &mut impl FnMut(&str) -> Result<Value>,
    path: &str,
    field: &str,
) -> Result<Vec<Value>> {
    let separator = if path.contains('?') { '&' } else { '?' };
    let mut all = Vec::new();
    for page in 1..=10_000 {
        let response = api(&format!("{path}{separator}per_page=100&page={page}"))?;
        let batch = response[field]
            .as_array()
            .with_context(|| format!("missing {field} list in GitHub response"))?;
        ensure!(batch.len() <= 100, "oversized GitHub page");
        all.extend_from_slice(batch);
        if batch.len() < 100 {
            return Ok(all);
        }
    }
    bail!("GitHub pagination did not terminate")
}

pub(super) fn unproven_jobs(jobs: &[Value]) -> Vec<&'static str> {
    REQUIRED_JOBS
        .into_iter()
        .filter(|name| {
            let prefix = format!("{name} (");
            let mut found = false;
            let all_green = jobs
                .iter()
                .filter(|job| {
                    job["name"]
                        .as_str()
                        .is_some_and(|n| n == *name || n.starts_with(&prefix))
                })
                .all(|job| {
                    found = true;
                    job["conclusion"].as_str() == Some("success")
                });
            !found || !all_green
        })
        .collect()
}

pub(super) fn verify_run(
    run: &str,
    identity: &Identity,
    api: &mut impl FnMut(&str) -> Result<Value>,
) -> Result<()> {
    ensure!(
        !run.is_empty() && run.bytes().all(|b| b.is_ascii_digit()),
        "invalid CI run ID"
    );
    let base = format!("repos/{}/actions/runs/{run}", identity.repository);
    ensure!(
        trusted_run(&api(&base)?, identity),
        "source must be a successful main push CI run for this repository and commit"
    );
    let jobs = pages(api, &format!("{base}/jobs?filter=latest"), "jobs")?;
    let missing = unproven_jobs(&jobs);
    ensure!(
        missing.is_empty(),
        "CI run did not run and pass every release job (missing or not green: {}); tag a commit whose main CI ran in full",
        missing.join(", ")
    );
    let artifacts = pages(api, &format!("{base}/artifacts"), "artifacts")?;
    for (target, _) in TARGETS {
        let name = format!("client-{target}");
        let matches: Vec<_> = artifacts
            .iter()
            .filter(|a| a["name"].as_str() == Some(&name))
            .collect();
        ensure!(
            matches.len() == 1 && matches[0]["expired"].as_bool() == Some(false),
            "missing/expired/ambiguous {target} package; rerun CI on main at this commit"
        );
    }
    Ok(())
}

pub(super) fn resolve(
    identity: &Identity,
    api: &mut impl FnMut(&str) -> Result<Value>,
) -> Result<String> {
    let response = api(&format!(
        "repos/{}/actions/workflows/ci.yml/runs?head_sha={}&event=push&branch=main&status=success&per_page=100",
        identity.repository, identity.commit
    ))?;
    let runs = response["workflow_runs"]
        .as_array()
        .context("missing workflow_runs list")?;
    for run in runs {
        if trusted_run(run, identity) {
            let id = run["id"]
                .as_u64()
                .context("invalid workflow run ID")?
                .to_string();
            // Deliberately fail on a newest matching run with incomplete gates;
            // do not silently fall back to another run's packages.
            verify_run(&id, identity, api)?;
            return Ok(id);
        }
    }
    bail!("no successful main CI at this commit; push and wait for CI before tagging")
}
