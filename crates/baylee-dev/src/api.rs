//! The API documents in `docs/api/` (`tools/api-docs`, oas-forge): `cargo
//! dev api` writes them, and `cargo dev api check` fails when one is stale or
//! a route its crate registers is not in it. The full gate runs the tool's
//! `gate`, which only reports a stale document while oas-forge is linked by
//! path (`tools/api-docs/src/main.rs` says why).
//!
//! Until oas-forge 0.2 is on crates.io the tool links a checkout beside this
//! one by path. Where there is none (CI), the step says so and passes, as
//! `cr-check` does without the rules. `cargo dev api serve` shows them in
//! Swagger UI on 127.0.0.1.
use crate::run::{cargo_with, code, status};
use std::{path::Path, process::Command};

/// The checkout the tool links by path.
const OAS_FORGE: &str = "../openapi-extract/Cargo.toml";

/// The tool's manifest, a workspace of its own.
const MANIFEST: &str = "tools/api-docs/Cargo.toml";

/// The tool's command for `task` (`write`, `check` or `gate`), or `None`, saying so,
/// when there is no oas-forge checkout to build it from.
pub fn command(task: &str) -> Option<Command> {
    if !Path::new(OAS_FORGE).is_file() {
        println!("STEP api SKIPPED - no {OAS_FORGE} (oas-forge 0.2 is not on crates.io yet)");
        return None;
    }
    let mut command = cargo_with(["run", "--quiet", "--manifest-path", MANIFEST, "--", task]);
    // A target of its own: the tool is outside the workspace, and sharing one
    // target would have each side's builds invalidate the other's.
    command.env("CARGO_TARGET_DIR", "target/api-docs");
    Some(command)
}

/// `cargo dev api [check | serve --port N]`: the tool with these arguments.
pub fn run(args: &[&str]) -> i32 {
    let Some((task, rest)) = args.split_first() else {
        return 2;
    };
    command(task).map_or(0, |mut command| code(status(command.args(rest))))
}
