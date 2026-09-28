//! `scripts/release/sign-archives.sh`, the release workflow's signing step,
//! run as the workflow runs it: with the real signer binary, a test key in
//! the secret's place, and GitHub's `GITHUB_REF_TYPE`. It signs every
//! archive and nothing else, fails a tag without the secret, and fails when
//! the secret is not a key the client trusts. And `release.yml` runs it
//! before it publishes.

#![cfg(unix)]

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use baylee_update::sign::{self, SigningKey};
use std::path::{Path, PathBuf};
use std::process::Output;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf()
}

fn scratch(tag: &str) -> PathBuf {
    static RUN: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "baylee-sign-script-{tag}-{}-{}",
        std::process::id(),
        RUN.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// What `actions/download-artifact` leaves in `out/`: five archives, each
/// with its `.sha256`.
fn release_dir(tag: &str) -> (PathBuf, Vec<PathBuf>) {
    let dir = scratch(tag);
    let mut archives = Vec::new();
    for target in [
        "x86_64-unknown-linux-gnu.tar.gz",
        "aarch64-unknown-linux-gnu.tar.gz",
        "x86_64-pc-windows-msvc.zip",
        "aarch64-pc-windows-msvc.zip",
        "aarch64-apple-darwin.zip",
    ] {
        let archive = dir.join(format!("baylee-client-0.1.0-beta.3-{target}"));
        std::fs::write(&archive, format!("archive for {target}")).unwrap();
        std::fs::write(format!("{}.sha256", archive.display()), "0000  whatever\n").unwrap();
        archives.push(archive);
    }
    (dir, archives)
}

const SEED: [u8; 32] = [42; 32];

fn seed_text() -> String {
    STANDARD.encode(SEED)
}

fn public_text() -> String {
    sign::public_text(&SigningKey::from_bytes(&SEED).verifying_key())
}

/// Runs the script on `dir` with exactly the environment given.
fn run(dir: &Path, env: &[(&str, &str)]) -> Output {
    let mut command = std::process::Command::new("bash");
    command
        .arg(root().join("scripts/release/sign-archives.sh"))
        .arg(dir)
        .env_clear()
        .env("PATH", std::env::var("PATH").unwrap_or_default())
        .env("BAYLEE_SIGNER", env!("CARGO_BIN_EXE_baylee-update-sign"));
    for (k, v) in env {
        command.env(k, v);
    }
    command.output().unwrap()
}

fn text(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

fn sigs(dir: &Path) -> Vec<String> {
    let mut sigs: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| Path::new(n).extension().is_some_and(|e| e == "sig"))
        .collect();
    sigs.sort();
    sigs
}

#[test]
fn a_tag_signs_every_archive_and_nothing_else() {
    let (dir, archives) = release_dir("tag");
    let out = run(
        &dir,
        &[
            ("GITHUB_REF_TYPE", "tag"),
            ("BAYLEE_UPDATE_SIGNING_KEY", &seed_text()),
            ("BAYLEE_UPDATE_VERIFY_KEY", &public_text()),
        ],
    );
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(sigs(&dir).len(), archives.len(), "{:?}", sigs(&dir));
    let key = SigningKey::from_bytes(&SEED).verifying_key();
    for archive in &archives {
        let sig = std::fs::read(format!("{}.sig", archive.display())).unwrap();
        let bytes = std::fs::read(archive).unwrap();
        assert_eq!(
            sign::verify(&bytes, &sig, &[key]),
            Ok(0),
            "{}",
            archive.display()
        );
    }
    // A checksum is not an archive.
    assert!(!sigs(&dir).iter().any(|s| s.contains(".sha256")));
    // The seed never reaches the log.
    assert!(!text(&out).contains(&seed_text()), "{}", text(&out));
}

#[test]
fn a_tag_without_the_secret_fails_and_signs_nothing() {
    let (dir, _) = release_dir("no-secret");
    let out = run(&dir, &[("GITHUB_REF_TYPE", "tag")]);
    assert!(!out.status.success(), "{}", text(&out));
    assert!(text(&out).contains("BAYLEE_UPDATE_SIGNING_KEY is not set"));
    assert!(sigs(&dir).is_empty());
    // Set but empty, as GitHub passes a secret that does not exist.
    let out = run(
        &dir,
        &[
            ("GITHUB_REF_TYPE", "tag"),
            ("BAYLEE_UPDATE_SIGNING_KEY", ""),
        ],
    );
    assert!(!out.status.success(), "{}", text(&out));
    assert!(sigs(&dir).is_empty());
}

/// `workflow_dispatch` is the dry run: without the secret it says so and
/// goes on, so a fork can still build.
#[test]
fn a_dry_run_without_the_secret_signs_nothing_and_goes_on() {
    let (dir, _) = release_dir("dry");
    let out = run(&dir, &[("GITHUB_REF_TYPE", "branch")]);
    assert!(out.status.success(), "{}", text(&out));
    assert!(text(&out).contains("::warning::"));
    assert!(sigs(&dir).is_empty());
}

/// A secret that is not the key the client is compiled with signs, and
/// then fails the release: the players' updaters would refuse every
/// archive.
#[test]
fn a_secret_the_client_does_not_trust_fails_the_release() {
    let (dir, _) = release_dir("untrusted");
    let out = run(
        &dir,
        &[
            ("GITHUB_REF_TYPE", "tag"),
            ("BAYLEE_UPDATE_SIGNING_KEY", &seed_text()),
        ],
    );
    assert!(!out.status.success(), "{}", text(&out));
    assert!(text(&out).contains("no trusted key"), "{}", text(&out));
}

#[test]
fn a_malformed_secret_fails_the_release() {
    let (dir, _) = release_dir("malformed");
    let out = run(
        &dir,
        &[
            ("GITHUB_REF_TYPE", "tag"),
            ("BAYLEE_UPDATE_SIGNING_KEY", "not a key"),
        ],
    );
    assert!(!out.status.success(), "{}", text(&out));
    assert!(sigs(&dir).is_empty());
}

#[test]
fn no_archive_at_all_fails() {
    let dir = scratch("empty");
    let out = run(
        &dir,
        &[
            ("GITHUB_REF_TYPE", "tag"),
            ("BAYLEE_UPDATE_SIGNING_KEY", &seed_text()),
        ],
    );
    assert!(!out.status.success(), "{}", text(&out));
}

/// The workflow runs the script on the tag with the secret, and publishes
/// only after it, refusing an archive without its signature.
#[test]
fn the_release_workflow_signs_before_it_publishes() {
    let workflow = std::fs::read_to_string(root().join(".github/workflows/release.yml")).unwrap();
    // A job is its `  name:` line and every line up to the next one
    // indented the same.
    let lines: Vec<&str> = workflow.lines().collect();
    let job = |name: &str| -> String {
        let head = format!("  {name}:");
        let start = lines
            .iter()
            .position(|l| *l == head)
            .unwrap_or_else(|| panic!("no job {name}"));
        let end = lines[start + 1..]
            .iter()
            .position(|l| l.starts_with("  ") && !l.starts_with("   "))
            .map_or(lines.len(), |i| start + 1 + i);
        lines[start..end].join("\n")
    };
    let sign = job("sign");
    assert!(sign.contains("scripts/release/sign-archives.sh"), "{sign}");
    assert!(
        sign.contains("BAYLEE_UPDATE_SIGNING_KEY: ${{ secrets.BAYLEE_UPDATE_SIGNING_KEY }}"),
        "{sign}"
    );
    assert!(!workflow.contains("BAYLEE_UPDATE_VERIFY_KEY"));
    let publish = job("publish");
    assert!(
        publish.contains("needs: [version, build, sign]"),
        "{publish}"
    );
    assert!(publish.contains(".sig"), "{publish}");
    // Only the release build may replace itself (the client's
    // `update::native::is_release_build`): the build step says so.
    let build =
        std::fs::read_to_string(root().join(".github/workflows/client-packages.yml")).unwrap();
    assert!(
        build.contains("BAYLEE_RELEASE_BUILD: '1'")
            && build.contains("cargo build --locked --workspace --bins --profile dist"),
        "{build}"
    );
}
