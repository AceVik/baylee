//! Deploys the committed HEAD to a production server by hand:
//! `BAYLEE_DEPLOY_HOST=user@host cargo dev deploy [--force]`.
//!
//! A release deploys itself: a timer on the server runs `baylee-deploy watch`
//! every minute and stages the newest `v*` tag. What a deploy does while
//! games are open is the server half's business (`baylee-deploy`, crate
//! baylee-ops): agent and engine are swapped at once, the gateway once the
//! last game is over, and `--force` ends the games instead of waiting.
//!
//! The server builds for itself, so nothing here cross-compiles. The source
//! travels as a `git bundle`, not a tarball: the build stamps its commit by
//! asking git, and a tree without `.git` says "unknown" in `/info`. After the
//! first run a bundle carries only the commits the server lacks. Only commits
//! are deployed; uncommitted edits stay here, and that is said, not refused.
use crate::run;
use anyhow::{Context, Result, bail};
use std::process::Command;

/// Where the server keeps its checkout (`baylee-deploy`'s `$root/src`).
const SERVER_SOURCE: &str = "/opt/baylee/src";
/// Where the deploy command for a hand deploy is built, apart from the
/// checkout's own target directory.
const DEPLOY_TARGET: &str = "/opt/baylee/state/deploy-target";

pub fn run(force: bool) -> Result<i32> {
    let host = run::require_env(
        "BAYLEE_DEPLOY_HOST",
        "the server to deploy to, as user@host",
    )?;
    let head = local(&["rev-parse", "HEAD"])?;
    if !local(&["status", "--porcelain", "--untracked-files=no"])?.is_empty() {
        eprintln!(
            "note: uncommitted changes are not deployed; building {}",
            local(&["rev-parse", "--short", "HEAD"])?
        );
    }

    let present = ssh(
        &host,
        &format!("git -C {SERVER_SOURCE} cat-file -e {head}^{{commit}} 2>/dev/null"),
    )?;
    if !present.status.success() {
        send_bundle(&host)?;
    }

    // The server half is taken from the commit being deployed, so a change to
    // how deploys work ships with the first deploy that needs it. It is built
    // from an archive of that commit in a directory of its own, never in the
    // shared checkout, which the minute timer's `baylee-deploy watch` may be
    // using; its own target directory keeps the rebuild incremental.
    let flag = if force { " --force" } else { "" };
    let stage = format!(
        "set -e; d=$(mktemp -d /tmp/baylee-deploy.XXXXXX); trap 'rm -rf \"$d\"' EXIT; \
         git -C {SERVER_SOURCE} archive {head} | tar -x -C \"$d\"; \
         \"${{CARGO:-$HOME/.cargo/bin/cargo}}\" build -q --release --locked --manifest-path \"$d/Cargo.toml\" \
         -p baylee-ops --target-dir {DEPLOY_TARGET}; \
         {DEPLOY_TARGET}/release/baylee-deploy stage {head}{flag}"
    );
    Ok(run::code(
        Command::new("ssh").arg(&host).arg(stage).status().ok(),
    ))
}

/// Ships the commits the server lacks (everything when it has none of ours).
fn send_bundle(host: &str) -> Result<()> {
    let have = ssh(
        host,
        &format!("git -C {SERVER_SOURCE} rev-parse -q --verify HEAD 2>/dev/null || true"),
    )?;
    let have = String::from_utf8(have.stdout)?.trim().to_owned();
    let range = if !have.is_empty()
        && run::succeeds(Command::new("git").args(["merge-base", "--is-ancestor", &have, "HEAD"]))
    {
        format!("{have}..HEAD")
    } else {
        "HEAD".to_owned()
    };
    let bundle = std::env::temp_dir().join(format!("baylee-deploy-{}.bundle", std::process::id()));
    let result = (|| {
        let created = Command::new("git")
            .args(["bundle", "create"])
            .arg(&bundle)
            .arg(&range)
            .output()
            .context("running git bundle")?;
        if !created.status.success() {
            bail!("git bundle create {range} failed");
        }
        if !run::succeeds(
            Command::new("scp")
                .arg("-q")
                .arg(&bundle)
                .arg(format!("{host}:/tmp/baylee-deploy.bundle")),
        ) {
            bail!("copying the bundle to {host} failed");
        }
        let fetched = ssh(
            host,
            &format!(
                "git -C {SERVER_SOURCE} fetch -q /tmp/baylee-deploy.bundle HEAD && rm /tmp/baylee-deploy.bundle"
            ),
        )?;
        if !fetched.status.success() {
            bail!("the server could not fetch the bundle");
        }
        Ok(())
    })();
    let _ = std::fs::remove_file(&bundle);
    result
}

fn ssh(host: &str, remote: &str) -> Result<std::process::Output> {
    Command::new("ssh")
        .arg(host)
        .arg(remote)
        .output()
        .context("running ssh")
}

fn local(args: &[&str]) -> Result<String> {
    let out = Command::new("git").args(args).output()?;
    if !out.status.success() {
        bail!("git {} failed", args.join(" "));
    }
    Ok(String::from_utf8(out.stdout)?.trim().to_owned())
}
