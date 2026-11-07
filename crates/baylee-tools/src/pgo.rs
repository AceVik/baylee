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

mod compare;
mod train;

/// The draft release that holds the client's profiles.
const PROFILES: &str = "client-profiles";

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
        ["client"] => client(root, &target, false),
        ["client", "--publish"] => client(root, &target, true),
        ["client", "ship", profile] => ship(root, &target, Path::new(profile)),
        ["compare", plain, profiled, ref rest @ ..] => {
            let rounds = rest.first().and_then(|rounds| rounds.parse().ok());
            compare::run(
                root,
                Path::new(plain),
                Path::new(profiled),
                rounds.unwrap_or(4),
            )
        }
        ["train", binary, ref rest @ ..] => {
            let work = target.join("pgo-profiles/client");
            fs::create_dir_all(work.join("raw"))?;
            let only = rest.first().and_then(|scene| scene.parse().ok());
            train::scenes(
                root,
                Path::new(binary),
                &work.join("raw"),
                &work.join("scenes"),
                only,
            )
        }
        _ => bail!(
            "usage: cargo tools pgo engine-server | client [--publish | ship <profile>] | train <client> [scene] | compare <plain> <profiled> [rounds]"
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

fn client(root: &Path, target: &Path, publish: bool) -> Result<()> {
    let plan = client_plan(root, target)?;
    let commit = if publish {
        Some(clean_commit(root)?)
    } else {
        None
    };
    let built = plan.build(|client, raw| {
        train::scenes(root, client, raw, &plan.work.join("scenes"), None)
            .map_err(|why| Failure(format!("the training: {why:#}")))
    })?;
    report(&built);
    if let Some(commit) = commit {
        upload(&plan, &built.profile, &commit)?;
    }
    Ok(())
}

/// Builds what ships from a profile, and stands it where `cargo build-client`
/// would have: `target/<triple>/dist`, which `cargo tools package client` reads.
/// It is built in a directory of its own because a used one, a release row's
/// warm cache, would keep code the profile never reached.
fn ship(root: &Path, target: &Path, profile: &Path) -> Result<()> {
    let plan = client_plan(root, target)?;
    let built = plan.ship(profile, &target.join("pgo-client/profiled"))?;
    report(&built);
    let place = target.join(&plan.triple).join(&plan.profile);
    stand_binaries(&built.binaries, &place)?;
    println!("{}", place.display());
    Ok(())
}

/// Copies what a package takes: the three binaries and their symbols.
fn stand_binaries(from: &Path, to: &Path) -> Result<()> {
    fs::create_dir_all(to)?;
    for name in ["baylee-client", "baylee-launch", "baylee-seat"] {
        let binary = format!("{name}{}", std::env::consts::EXE_SUFFIX);
        fs::copy(from.join(&binary), to.join(&binary))
            .with_context(|| format!("standing {binary} in {}", to.display()))?;
    }
    let pdb = from.join("baylee_client.pdb");
    if pdb.is_file() {
        fs::copy(&pdb, to.join("baylee_client.pdb"))?;
    }
    let dsym = from.join("baylee-client.dSYM");
    if dsym.is_dir() {
        let kept = to.join("baylee-client.dSYM");
        let _ = fs::remove_dir_all(&kept);
        copy_tree(&fs::canonicalize(&dsym)?, &kept)?;
    }
    Ok(())
}

fn copy_tree(from: &Path, to: &Path) -> Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let place = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &place)?;
        } else {
            fs::copy(entry.path(), place)?;
        }
    }
    Ok(())
}

/// The commit a profile will belong to. A changed file would make it a profile
/// of something no commit holds, so this is asked before the long build.
fn clean_commit(root: &Path) -> Result<String> {
    let git = |args: &[&str]| -> Result<String> {
        let output = Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .context("asking git")?;
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
    };
    if !git(&["status", "--porcelain", "--untracked-files=no"])?.is_empty() {
        bail!("a profile belongs to a commit: this checkout has changes");
    }
    git(&["rev-parse", "HEAD"])
}

/// Hands the profile to the release: an asset of the draft `client-profiles`,
/// named by target and commit. A draft, because the site and the updater read
/// the published list; only the job that fetches it may see a draft.
fn upload(plan: &ClientPlan, profile: &Path, commit: &str) -> Result<()> {
    let named = plan
        .work
        .join(format!("client-{}-{commit}.profdata", plan.triple));
    fs::copy(profile, &named)?;
    let gh = |args: &[&str]| {
        Command::new("gh")
            .args(args)
            .current_dir(&plan.root)
            .status()
            .context("running gh")
    };
    if !gh(&["release", "view", PROFILES])?.success() {
        let made = gh(&[
            "release",
            "create",
            PROFILES,
            "--draft",
            "--title",
            "Client profiles",
            "--notes",
            "Profiles for the client, one per target and commit (docs/releasing.md).",
        ])?;
        if !made.success() {
            bail!("the draft {PROFILES} could not be made");
        }
    }
    let path = named.to_string_lossy();
    if !gh(&["release", "upload", PROFILES, &path, "--clobber"])?.success() {
        bail!("the profile could not be uploaded");
    }
    println!("uploaded {path}");
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
