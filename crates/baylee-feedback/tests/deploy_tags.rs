//! Signed release tags (`docs/deploy-hooks.md` §"Signed release tags"):
//! with an `OpenPGP` keyring in the deploy's settings, `watch` and a `stage`
//! of a tag deploy only a tag that key signed; without one they warn and
//! deploy as before. Real `git` and `gpg` against a scratch repository;
//! skipped where `gpg` is not installed.

#![cfg(unix)]

mod hooks_harness;

use std::os::unix::fs::PermissionsExt as _;
use std::path::Path;
use std::process::Command;

use hooks_harness::{A, Server, which};

/// Runs a program, or panics with what it said.
fn run(program: &Path, args: &[&str], cwd: &Path, gnupg: Option<&Path>) {
    let mut command = Command::new(program);
    // None of this machine's own git settings (a signing program, say).
    command
        .args(args)
        .current_dir(cwd)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1");
    if let Some(home) = gnupg {
        command.env("GNUPGHOME", home);
    }
    let out = command.output().expect("runs");
    assert!(
        out.status.success(),
        "{} {args:?}: {}",
        program.display(),
        String::from_utf8_lossy(&out.stderr)
    );
}

/// A `GnuPG` home with one fresh signing key for `email`, and that key's
/// public half exported to `export`.
fn key(gpg: &Path, home: &Path, email: &str, export: &Path) {
    std::fs::create_dir_all(home).unwrap();
    std::fs::set_permissions(home, std::fs::Permissions::from_mode(0o700)).unwrap();
    let uid = format!("Release <{email}>");
    run(
        gpg,
        &[
            "--batch",
            "--pinentry-mode",
            "loopback",
            "--passphrase",
            "",
            "--quick-gen-key",
            &uid,
            "ed25519",
            "sign",
            "never",
        ],
        home,
        Some(home),
    );
    let out = Command::new(gpg)
        .args(["--batch", "--export", email])
        .env("GNUPGHOME", home)
        .output()
        .unwrap();
    std::fs::write(export, out.stdout).unwrap();
}

/// The server's source is a real repository with `v1.0.0` signed by the
/// release key, `v1.0.1` unsigned and `v1.0.2` signed by another key; the
/// release key is in `signer.gpg`, the other in `other.gpg`.
fn signed_server() -> Option<Server> {
    let gpg = which("gpg");
    if !gpg.is_file() {
        eprintln!("no gpg here: signed-tag cases skipped");
        return None;
    }
    let git = which("git");
    let server = Server::bare();
    let keys = server.scratch.join("keys");
    std::fs::create_dir_all(&keys).unwrap();
    let (signer, other) = (keys.join("signer"), keys.join("other"));
    key(
        &gpg,
        &signer,
        "signer@example.invalid",
        &keys.join("signer.gpg"),
    );
    key(
        &gpg,
        &other,
        "other@example.invalid",
        &keys.join("other.gpg"),
    );
    let src = server.root.join("src");
    let me = [
        "-c",
        "user.name=Release",
        "-c",
        "user.email=signer@example.invalid",
    ];
    run(&git, &["init", "-q"], &src, None);
    let mut commit = me.to_vec();
    commit.extend(["commit", "-q", "--allow-empty", "-m", "one"]);
    run(&git, &commit, &src, None);
    for (tag, home, signing) in [
        ("v1.0.0", Some(&signer), Some("signer@example.invalid")),
        ("v1.0.1", None, None),
        ("v1.0.2", Some(&other), Some("other@example.invalid")),
    ] {
        let mut args = me.to_vec();
        let key_setting;
        if let Some(signing) = signing {
            key_setting = format!("user.signingkey={signing}");
            args.extend(["-c", &key_setting, "tag", "-s", "-m", tag, tag]);
        } else {
            args.extend(["tag", "-a", "-m", tag, tag]);
        }
        run(&git, &args, &src, home.map(std::path::PathBuf::as_path));
    }
    for home in [&signer, &other] {
        let _ = Command::new(which("gpgconf"))
            .args(["--kill", "all"])
            .env("GNUPGHOME", home)
            .output();
    }
    Some(server)
}

/// Puts `which` (`signer.gpg`, `other.gpg`) in as the deploy's keyring,
/// root's (or the test user's) `0644`.
fn keyring(server: &Server, which: &str) -> std::path::PathBuf {
    let path = server.etc.join("release-keyring.gpg");
    std::fs::copy(server.scratch.join("keys").join(which), &path).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
    path
}

fn watch_sees(server: &Server, tag: &str) -> hooks_harness::Ran {
    server.set("tags", &format!("{A}\trefs/tags/{tag}"));
    server.run(&["watch"])
}

#[test]
fn without_a_keyring_a_tag_is_deployed_with_a_warning() {
    let Some(server) = signed_server() else {
        return;
    };
    let ran = watch_sees(&server, "v1.0.1");
    assert!(ran.ok, "{}", ran.said);
    assert!(ran.said.contains("is not verified"), "{}", ran.said);
    assert_eq!(server.deployed().as_deref(), Some(A));
}

#[test]
fn a_tag_signed_by_the_configured_key_is_deployed() {
    let Some(server) = signed_server() else {
        return;
    };
    keyring(&server, "signer.gpg");
    let ran = watch_sees(&server, "v1.0.0");
    assert!(ran.ok, "{}", ran.said);
    assert!(ran.said.contains("signature verified"), "{}", ran.said);
    assert_eq!(server.deployed().as_deref(), Some(A));
}

#[test]
fn an_unsigned_or_foreign_tag_is_refused_before_anything_is_built() {
    let Some(server) = signed_server() else {
        return;
    };
    keyring(&server, "signer.gpg");
    for tag in ["v1.0.1", "v1.0.2"] {
        let ran = watch_sees(&server, tag);
        assert!(
            ran.said.contains("refusing release tag"),
            "{tag}: {}",
            ran.said
        );
        assert!(ran.first("cargo build").is_none(), "{tag}: {}", ran.calls);
        assert_eq!(server.deployed(), None, "{tag}");
        // Not tried again every minute.
        assert_eq!(server.state("last-release").as_deref(), Some(tag));
    }
    // Staged by hand, by name: refused the same way.
    let ran = server.run(&["stage", "v1.0.2"]);
    assert!(!ran.ok, "{}", ran.said);
    assert!(ran.first("cargo build").is_none(), "{}", ran.calls);

    // The right tag with the other key as the keyring: refused too.
    keyring(&server, "other.gpg");
    let ran = watch_sees(&server, "v1.0.0");
    assert!(ran.said.contains("refusing release tag"), "{}", ran.said);
    assert_eq!(server.deployed(), None);
}

#[test]
fn a_keyring_anyone_could_have_written_is_refused() {
    let Some(server) = signed_server() else {
        return;
    };
    let path = keyring(&server, "signer.gpg");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o664)).unwrap();
    let ran = watch_sees(&server, "v1.0.0");
    assert!(ran.said.contains("not root's alone"), "{}", ran.said);
    assert_eq!(server.deployed(), None);
}
