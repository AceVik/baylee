//! The API documents in `docs/api/` (`tools/api-docs`, oas-forge):
//! `cargo dev api` writes them, `cargo dev api check` (also the full gate's
//! `api` step, and CI's `validate` job) fails when one is stale or a route a
//! service registers is in none of them, and `cargo dev api serve` shows
//! them in Swagger UI on 127.0.0.1.
//!
//! The tool is a workspace of its own (its `Cargo.toml` says why), so no
//! `--workspace` run builds or tests it: the gate's `api-tests` step does.
use crate::run::{cargo_with, code, status};
use std::process::Command;

/// The tool's manifest, a workspace of its own.
const MANIFEST: &str = "tools/api-docs/Cargo.toml";

/// `cargo <verb> --locked --manifest-path tools/api-docs/Cargo.toml <rest>`,
/// in a target of its own: sharing the workspace's would have each side's
/// builds invalidate the other's.
fn cargo<const N: usize>(verb: &str, rest: [&str; N]) -> Command {
    let mut command = cargo_with([verb, "--quiet", "--locked", "--manifest-path", MANIFEST]);
    command
        .args(rest)
        .env("CARGO_TARGET_DIR", "target/api-docs");
    command
}

/// The tool's command for `task` (`write`, `check` or `serve`).
pub fn command(task: &str) -> Command {
    cargo("run", ["--", task])
}

/// The tool's own tests.
pub fn tests() -> Command {
    cargo("test", [])
}

/// `cargo dev api [check | serve --port N]`: the tool with these arguments.
pub fn run(args: &[&str]) -> i32 {
    let Some((task, rest)) = args.split_first() else {
        return 2;
    };
    code(status(command(task).args(rest)))
}
