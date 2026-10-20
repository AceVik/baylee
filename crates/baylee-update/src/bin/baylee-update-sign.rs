//! Signs release archives, and checks that the client would accept them.
//!
//!     baylee-update-sign sign <archive>...
//!     baylee-update-sign verify [--key <base64>]... <archive>...
//!     baylee-update-sign release <dir>
//!
//! `sign` reads the seed from `BAYLEE_UPDATE_SIGNING_KEY` (base64 of 32
//! bytes, the Actions secret) and writes `<archive>.sig` beside each
//! archive. `verify` checks every `<archive>.sig` against the keys the
//! client is compiled with (`baylee_update::sign::TRUSTED_KEYS`), or against
//! the `--key`s given instead; the release workflow runs it after signing,
//! so a secret that does not match the client's keys fails the release
//! rather than every player's update. The seed is never printed.
//!
//! `release` is the release workflow's step (#326, `docs/releasing.md`
//! §"Signing"): it signs every `.zip` and `.tar.gz` in `<dir>`, verifies each
//! signature as `verify` does and checks that none is missing. Without the
//! secret, a tag (`GITHUB_REF_TYPE=tag`) fails, because a release is never
//! published unsigned again, and a dry run says so and signs nothing.
//! `BAYLEE_UPDATE_VERIFY_KEY` replaces the compiled keys for that check; only
//! this binary's tests set it, the release workflow never does.

use baylee_update::sign;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.split_first() {
        Some((command, rest)) if command == "sign" => sign_all(rest),
        Some((command, rest)) if command == "verify" => verify_all(rest),
        Some((command, [dir])) if command == "release" => release(std::path::Path::new(dir)),
        _ => Err("usage: baylee-update-sign sign|verify [--key <base64>]... <archive>... | release <dir>".into()),
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

/// Signs and verifies a release directory's archives (see the module doc).
fn release(dir: &std::path::Path) -> Result<(), String> {
    let mut archives: Vec<String> = std::fs::read_dir(dir)
        .map_err(|err| format!("{}: {err}", dir.display()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| {
            let name = path.to_string_lossy();
            name.ends_with(".zip") || name.ends_with(".tar.gz")
        })
        .map(|path| path.display().to_string())
        .collect();
    archives.sort();
    if archives.is_empty() {
        println!("::error::no release archive in {}", dir.display());
        return Err("no release archive".into());
    }
    if std::env::var("BAYLEE_UPDATE_SIGNING_KEY")
        .unwrap_or_default()
        .trim()
        .is_empty()
    {
        if std::env::var("GITHUB_REF_TYPE").as_deref() == Ok("tag") {
            println!(
                "::error::BAYLEE_UPDATE_SIGNING_KEY is not set; a tag never publishes unsigned archives"
            );
            return Err("BAYLEE_UPDATE_SIGNING_KEY is not set".into());
        }
        println!("::warning::BAYLEE_UPDATE_SIGNING_KEY is not set; this dry run signs nothing");
        return Ok(());
    }
    sign_all(&archives)?;
    let mut verify = Vec::new();
    if let Ok(key) = std::env::var("BAYLEE_UPDATE_VERIFY_KEY")
        && !key.is_empty()
    {
        verify.extend(["--key".to_owned(), key]);
    }
    verify.extend(archives.iter().cloned());
    verify_all(&verify)?;
    for archive in &archives {
        let signed = std::fs::metadata(format!("{archive}.sig")).is_ok_and(|m| m.len() > 0);
        if !signed {
            println!("::error::{archive} has no signature");
            return Err(format!("{archive} has no signature"));
        }
    }
    println!("signed and verified {} archives", archives.len());
    Ok(())
}
