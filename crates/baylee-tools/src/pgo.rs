//! `cargo tools pgo`: a profile-guided build by hand.
//!
//! The steps are `baylee_pgo`'s. `engine-server` builds the engine server from
//! a profile of the checkout as it stands and proves it against the plain build,
//! the steps a deploy runs. `client` builds the client from a profile made here,
//! on this machine's screen and GPU ([`train`]), and stops without a binary when
//! a step fails or the profile did not reach what ships. Both print where the
//! binaries are.
//!
//! `LLVM_PROFDATA` names the tool where the toolchain has no `llvm-tools`.

mod train;

use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context as _, Result, bail};
use baylee_pgo::{ClientBuild, ClientPlan, Failure, Plan};

pub fn run(root: &Path, args: &[String]) -> Result<()> {
    let target = root.join("target");
    match args.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        ["engine-server"] => engine_server(root, &target),
        ["client"] => client(root, &target),
        ["client", "ship", profile] => ship(root, &target, Path::new(profile)),
        ["train", binary, ref rest @ ..] => {
            let work = target.join("pgo-profiles/client");
            fs::create_dir_all(work.join("raw"))?;
            let only = rest.first().and_then(|scene| scene.parse().ok());
            train::scenes(
                Path::new(binary),
                &work.join("raw"),
                &work.join("scenes"),
                only,
            )
        }
        _ => bail!(
            "usage: cargo tools pgo engine-server | client [ship <profile>] | train <client> [scene]"
        ),
    }
}

fn cargo() -> PathBuf {
    std::env::var_os("CARGO")
        .unwrap_or_else(|| OsString::from("cargo"))
        .into()
}

fn rustc() -> OsString {
    std::env::var_os("RUSTC").unwrap_or_else(|| OsString::from("rustc"))
}

fn profdata() -> Result<PathBuf> {
    Ok(match std::env::var_os("LLVM_PROFDATA") {
        Some(tool) => tool.into(),
        None => baylee_pgo::profdata(&rustc())?,
    })
}

fn engine_server(root: &Path, target: &Path) -> Result<()> {
    let plan = Plan {
        cargo: cargo(),
        profdata: profdata()?,
        wrapper: std::env::current_exe().context("the running program")?,
        root: root.to_owned(),
        work: target.join("pgo-profiles/by-hand"),
        targets: target.join("pgo"),
        plain: target.to_owned(),
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

fn client_plan(root: &Path, target: &Path) -> Result<ClientPlan> {
    Ok(ClientPlan {
        cargo: cargo(),
        profdata: profdata()?,
        wrapper: std::env::current_exe().context("the running program")?,
        root: root.to_owned(),
        work: target.join("pgo-profiles/client"),
        targets: target.join("pgo-client"),
        profile: "dist".to_owned(),
        triple: host()?,
    })
}

fn client(root: &Path, target: &Path) -> Result<()> {
    let plan = client_plan(root, target)?;
    let built = plan.build(|client, raw| {
        train::every_scene(client, raw, &plan.work.join("scenes"))
            .map_err(|why| Failure(format!("the training: {why:#}")))
    })?;
    report(&built);
    Ok(())
}

/// Builds what ships from a profile made on another run of `client`.
fn ship(root: &Path, target: &Path, profile: &Path) -> Result<()> {
    let plan = client_plan(root, target)?;
    let built = plan.ship(profile, &target.join("pgo-client/profiled"))?;
    report(&built);
    Ok(())
}

fn report(built: &ClientBuild) {
    println!(
        "{} functions, {} without counts; the client's own crate {} and {}",
        built.functions, built.unmatched, built.own_functions, built.own_unmatched
    );
    println!("{}", built.binaries.display());
}

/// The triple this machine builds for: the one its training can run.
fn host() -> Result<String> {
    let output = Command::new(rustc())
        .arg("-vV")
        .output()
        .context("asking rustc")?;
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .map(str::to_owned)
        .context("rustc names no host")
}
