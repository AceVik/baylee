//! Signs release archives, and checks that the client would accept them.
//!
//!     baylee-update-sign sign <archive>...
//!     baylee-update-sign verify [--key <base64>]... <archive>...
//!
//! `sign` reads the seed from `BAYLEE_UPDATE_SIGNING_KEY` (base64 of 32
//! bytes, the Actions secret) and writes `<archive>.sig` beside each
//! archive. `verify` checks every `<archive>.sig` against the keys the
//! client is compiled with (`baylee_update::sign::TRUSTED_KEYS`), or against
//! the `--key`s given instead; the release workflow runs it after signing,
//! so a secret that does not match the client's keys fails the release
//! rather than every player's update. The seed is never printed.
//! `scripts/release/sign-archives.sh` is the workflow's door to both.

use baylee_update::sign;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.split_first() {
        Some((command, rest)) if command == "sign" => sign_all(rest),
        Some((command, rest)) if command == "verify" => verify_all(rest),
        _ => Err("usage: baylee-update-sign sign|verify [--key <base64>]... <archive>...".into()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("baylee-update-sign: {err}");
            ExitCode::FAILURE
        }
    }
}

fn sign_all(files: &[String]) -> Result<(), String> {
    if files.is_empty() {
        return Err("no archive to sign".into());
    }
    let seed = std::env::var("BAYLEE_UPDATE_SIGNING_KEY").unwrap_or_default();
    if seed.trim().is_empty() {
        return Err("BAYLEE_UPDATE_SIGNING_KEY is not set".into());
    }
    let key =
        sign::signing_key(&seed).map_err(|err| format!("BAYLEE_UPDATE_SIGNING_KEY: {err}"))?;
    println!(
        "signing with public key {}",
        sign::public_text(&key.verifying_key())
    );
    for file in files {
        let bytes = std::fs::read(file).map_err(|err| format!("{file}: {err}"))?;
        std::fs::write(format!("{file}.sig"), sign::sign(&key, &bytes))
            .map_err(|err| format!("{file}.sig: {err}"))?;
        println!("signed {file}");
    }
    Ok(())
}

fn verify_all(args: &[String]) -> Result<(), String> {
    let mut keys = Vec::new();
    let mut files = Vec::new();
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        if arg == "--key" {
            let text = rest.next().ok_or("--key needs a value")?;
            keys.push(sign::public_key(text).map_err(|err| format!("--key: {err}"))?);
        } else {
            files.push(arg);
        }
    }
    if keys.is_empty() {
        keys = sign::trusted_keys();
    }
    if files.is_empty() {
        return Err("no archive to verify".into());
    }
    for file in files {
        let bytes = std::fs::read(file).map_err(|err| format!("{file}: {err}"))?;
        let sig =
            std::fs::read(format!("{file}.sig")).map_err(|err| format!("{file}.sig: {err}"))?;
        let which = sign::verify(&bytes, &sig, &keys).map_err(|err| format!("{file}: {err}"))?;
        println!("verified {file} (key {which})");
    }
    Ok(())
}
