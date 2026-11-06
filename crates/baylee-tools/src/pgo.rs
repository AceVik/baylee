//! `cargo tools pgo`: a profile-guided build by hand.
//!
//! The steps are `baylee_pgo`'s, the ones a deploy runs. `engine-server` builds
//! the engine server from a profile of the checkout as it stands and proves it
//! against the plain build; it prints where the binary is.
//!
//! `LLVM_PROFDATA` names the tool where the toolchain has no `llvm-tools`.

use std::{ffi::OsString, path::Path};

use anyhow::{Context as _, Result, bail};
use baylee_pgo::Plan;

pub fn run(root: &Path, args: &[String]) -> Result<()> {
    let ["engine-server"] = args.iter().map(String::as_str).collect::<Vec<_>>()[..] else {
        bail!("usage: cargo tools pgo engine-server");
    };
    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| OsString::from("rustc"));
    let profdata = match std::env::var_os("LLVM_PROFDATA") {
        Some(tool) => tool.into(),
        None => baylee_pgo::profdata(&rustc)?,
    };
    let target = root.join("target");
    let plan = Plan {
        cargo: std::env::var_os("CARGO")
            .unwrap_or_else(|| OsString::from("cargo"))
            .into(),
        profdata,
        wrapper: std::env::current_exe().context("the running program")?,
        root: root.to_owned(),
        work: target.join("pgo-profiles/by-hand"),
        targets: target.join("pgo"),
        plain: target.clone(),
        profile: "release".to_owned(),
        ship: [
            "-p",
            "baylee-engine-server",
            "--bin",
            "baylee-engine-server",
        ]
        .map(str::to_owned)
        .to_vec(),
    };
    println!("{}", plan.build()?.display());
    Ok(())
}
