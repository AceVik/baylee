//! Bounds the RUSTSEC-2026-0387 exception to the reviewed Ristretto-only SDK.
//!
//! See docs/central-login.md, "Bigint advisory scope". A changed source, version
//! or additional consumer requires a fresh review, not an inherited exception.
use crate::run::{self, Steps, cargo_with};
use anyhow::{Context, Result, ensure};
use serde_json::Value;
use std::{collections::BTreeSet, process::Command, time::Instant};

const CENTRAL_REV: &str = "6a208cf63b6e0a7a56ebe4b33e0f5718b32f4d90";
const REGISTRY: &str = "registry+https://github.com/rust-lang/crates.io-index";

/// Checks the immutable sources and every possible consumer of the exception.
pub(super) fn check(steps: &mut Steps) {
    let started = Instant::now();
    let checked = (|| -> Result<()> {
        let output =
            run::captured(&mut metadata_command()).context("could not run cargo metadata")?;
        ensure!(
            output.status.success(),
            "cargo metadata failed: {}",
            run::tail(&run::text(&output), 8)
        );
        verify(&serde_json::from_slice(&output.stdout)?)
    })();
    match &checked {
        Ok(()) => println!(
            "STEP crypto-bigint-scope ok (reviewed Central Ristretto pin and reverse graph, {}s)",
            started.elapsed().as_secs()
        ),
        Err(error) => println!("STEP crypto-bigint-scope FAILED: {error:#}"),
    }
    steps.record(checked.is_ok());
}

fn metadata_command() -> Command {
    // Resolve the superset, including inactive optional consumers. This runs
    // no build scripts and compiles none of the development configurations.
    cargo_with([
        "metadata",
        "--locked",
        "--all-features",
        "--format-version",
        "1",
    ])
}

fn verify(metadata: &Value) -> Result<()> {
    let packages = metadata["packages"]
        .as_array()
        .context("missing cargo packages")?;
    let nodes = metadata["resolve"]["nodes"]
        .as_array()
        .context("missing cargo resolution graph")?;
    let source =
        format!("git+https://github.com/AceVik/baylee-central.git?rev={CENTRAL_REV}#{CENTRAL_REV}");
    let central = reviewed(packages, "central-crypto", "0.1.0", &source)?;
    let opaque = reviewed(packages, "opaque-ke", "4.0.1", REGISTRY)?;
    let voprf = reviewed(packages, "voprf", "0.5.0", REGISTRY)?;
    let elliptic = reviewed(packages, "elliptic-curve", "0.13.8", REGISTRY)?;
    let bigint = reviewed(packages, "crypto-bigint", "0.5.5", REGISTRY)?;
    expect_callers(nodes, opaque, &[central])?;
    expect_callers(nodes, voprf, &[opaque])?;
    expect_callers(nodes, elliptic, &[opaque, voprf])?;
    expect_callers(nodes, bigint, &[elliptic])
}

fn reviewed<'a>(packages: &'a [Value], name: &str, version: &str, source: &str) -> Result<&'a str> {
    let mut candidates = packages.iter().filter(|p| p["name"].as_str() == Some(name));
    let package = candidates
        .next()
        .with_context(|| format!("missing {name}"))?;
    ensure!(
        candidates.next().is_none(),
        "multiple {name} versions need review"
    );
    ensure!(
        package["version"].as_str() == Some(version) && package["source"].as_str() == Some(source),
        "{name} changed from the reviewed version/source; review RUSTSEC-2026-0387 again"
    );
    package["id"].as_str().context("missing cargo package id")
}

fn expect_callers(nodes: &[Value], target: &str, expected: &[&str]) -> Result<()> {
    ensure!(
        nodes
            .iter()
            .filter(|node| node["id"].as_str() == Some(target))
            .count()
            == 1,
        "missing or duplicate resolution node for {target}"
    );
    let mut actual = BTreeSet::new();
    for node in nodes {
        let id = node["id"].as_str().context("missing resolution node id")?;
        let dependencies = node["dependencies"]
            .as_array()
            .context("missing resolution dependencies")?;
        for dependency in dependencies {
            let dependency = dependency.as_str().context("invalid dependency id")?;
            if dependency == target {
                actual.insert(id);
            }
        }
    }
    ensure!(
        actual == expected.iter().copied().collect::<BTreeSet<_>>(),
        "consumers of {target} changed; review RUSTSEC-2026-0387 again"
    );
    Ok(())
}

#[cfg(test)]
mod tests;
