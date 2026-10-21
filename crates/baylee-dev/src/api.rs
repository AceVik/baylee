//! The API documents in `docs/api/` (`crates/baylee-api-docs`, oas-forge):
//! `cargo dev api` writes them, `cargo dev api check` (also the full gate's
//! `api` step, and CI's `validate` job) fails when one is stale or a route a
//! service registers is in none of them, and `cargo dev api serve` shows
//! them in Swagger UI on 127.0.0.1.
use crate::run::{cargo_with, code, status};
use std::process::Command;

/// The tool's command for `task` (`write`, `check` or `serve`).
pub fn command(task: &str) -> Command {
    cargo_with([
        "run",
        "--quiet",
        "--locked",
        "-p",
        "baylee-api-docs",
        "--",
        task,
    ])
}

/// `cargo dev api [check | serve --port N]`: the tool with these arguments.
pub fn run(args: &[&str]) -> i32 {
    let Some((task, rest)) = args.split_first() else {
        return 2;
    };
    code(status(command(task).args(rest)))
}
