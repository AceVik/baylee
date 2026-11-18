//! Provisions the local development stack's Central and Tower databases
//! (`cargo dev stack up --central`, `docs/dev/commands.md` §"Running").
//!
//! `baylee-dev-provision central|tower` reads one JSON document on stdin and
//! writes one on stdout. Every secret travels that way, in memory, between
//! this process and the stack's supervisor; nothing is logged, and an error
//! names the step that failed and no value. Loopback development only: the
//! Central connection is `central_postgres`'s development one (numeric
//! loopback, no TLS), and the registration proof is lowered the way the
//! gateway's integration tests lower it.
#![forbid(unsafe_code)]

// The Tower content reads the compiled card registry.
use baylee_cards_pool as _;

mod central;
mod tower;

use anyhow::{Context as _, Result, bail};
use std::io::{Read as _, Write as _};

/// More than either request needs; a larger input is not one of ours.
const MAX_INPUT: u64 = 64 * 1024;

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("baylee-dev-provision: {error:#}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let mode = std::env::args().nth(1).unwrap_or_default();
    let mut input = Vec::new();
    std::io::stdin()
        .take(MAX_INPUT + 1)
        .read_to_end(&mut input)
        .context("reading the request")?;
    if input.len() as u64 > MAX_INPUT {
        bail!("the request is larger than {MAX_INPUT} bytes");
    }
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("starting the runtime")?;
    let reply = match mode.as_str() {
        "central" => {
            let request = parse(&input)?;
            serde_json::to_vec(&runtime.block_on(central::run(request))?)?
        }
        "tower" => {
            let request = parse(&input)?;
            serde_json::to_vec(&runtime.block_on(tower::run(request))?)?
        }
        _ => bail!("use `central` or `tower`, with the request on stdin"),
    };
    let mut output = std::io::stdout().lock();
    output.write_all(&reply)?;
    output.write_all(b"\n")?;
    Ok(())
}

/// A request that does not parse is refused without echoing any of it.
fn parse<T: serde::de::DeserializeOwned>(input: &[u8]) -> Result<T> {
    serde_json::from_slice(input).map_err(|error| {
        anyhow::anyhow!("the request is not the expected JSON ({})", shape(&error))
    })
}

/// Where a parse failed, without the offending value.
fn shape(error: &serde_json::Error) -> String {
    format!(
        "{:?} at line {} column {}",
        error.classify(),
        error.line(),
        error.column()
    )
}

#[cfg(test)]
mod tests;
