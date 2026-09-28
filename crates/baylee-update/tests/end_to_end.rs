//! The whole updater against a stub of GitHub on the loopback: a releases
//! answer, an archive built and signed here with a test key, its checksum.
//! Check, download, verify, stage, apply "on exit", announce at the next
//! start; and each way it must refuse, with nothing changed.

mod support;

use baylee_update::apply::{self, Install};
use baylee_update::check::{Checker, Context, Manual, Outcome};
use baylee_update::plan::{NEW, Os};
use baylee_update::service::{Command, Service, Settings};
use baylee_update::sign::{self, SigningKey, VerifyingKey};
use sha2::{Digest as _, Sha256};
use std::time::Duration;
use support::{Answer, Stub, expected, installed, snapshot};

const OLD: &str = "0.1.0-beta.2";
const NEWER: &str = "0.1.0-beta.3";

fn test_key() -> SigningKey {
    SigningKey::from_bytes(&[7; 32])
}

fn stranger() -> SigningKey {
    SigningKey::from_bytes(&[9; 32])
}

/// How a release is published on the stub.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Publish {
    /// Authentic old release relabelled as the advertised newer version.
    WrongVersion,
    /// Authentic same-OS release for another CPU relabelled as this target.
    WrongArchitecture,
    /// Archive, `.sig` by the test key, `.sha256`.
    Signed,
    /// No `.sig` at all.
    Unsigned,
    /// Signed by a key the client does not trust.
    SignedByAStranger,
    /// Signed, then one byte of the archive changed and its checksum
    /// recomputed: someone who could replace both.
    Tampered,
    /// Signed, with a checksum of other bytes: a download cut short or
    /// mixed up.
    BadChecksum,
}

/// Serves a releases list with `NEWER` for `os` and its files.
fn publish(stub: &Stub, os: Os, how: Publish) {
    let name = baylee_update::release::archive_name(NEWER, support::target(os));
    let (_, mut bytes) = match how {
        Publish::WrongVersion => support::archive(os, OLD),
        Publish::WrongArchitecture => {
            support::archive_for_target(os, NEWER, "aarch64-unknown-linux-gnu")
        }
        _ => support::archive(os, NEWER),
    };
    let signer = if how == Publish::SignedByAStranger {
        stranger()
    } else {
        test_key()
    };
    let signature = sign::sign(&signer, &bytes);
    if how == Publish::Tampered {
        let middle = bytes.len() / 2;
        bytes[middle] ^= 0x20;
    }
    let digest = if how == Publish::BadChecksum {
        sign::hex_of(&Sha256::digest(b"other bytes"))
    } else {
        sign::hex_of(&Sha256::digest(&bytes))
    };
    let mut assets = vec![serde_json::json!({
        "name": name, "size": bytes.len(),
        "browser_download_url": format!("{}/dl/{name}", stub.base),
    })];
    assets.push(serde_json::json!({
        "name": format!("{name}.sha256"), "size": 100,
        "browser_download_url": format!("{}/dl/{name}.sha256", stub.base),
    }));
    if how != Publish::Unsigned {
        assets.push(serde_json::json!({
            "name": format!("{name}.sig"), "size": 89,
            "browser_download_url": format!("{}/dl/{name}.sig", stub.base),
        }));
    }
    let releases = serde_json::json!([
        {"tag_name": format!("v{NEWER}"), "draft": false, "prerelease": true,
         "html_url": format!("{}/page/v{NEWER}", stub.base), "assets": assets},
        {"tag_name": format!("v{OLD}"), "draft": false, "prerelease": true,
         "html_url": format!("{}/page/v{OLD}", stub.base), "assets": []},
    ]);
    stub.route(
        "/releases",
        Answer {
            status: 200,
            headers: vec![("ETag".into(), "\"list-1\"".into())],
            body: releases.to_string().into_bytes(),
        },
    );
    stub.route(&format!("/dl/{name}"), Answer::ok(bytes));
    stub.route(
        &format!("/dl/{name}.sha256"),
        Answer::ok(format!("{digest}  {name}\n")),
    );
    stub.route(&format!("/dl/{name}.sig"), Answer::ok(signature));
}

fn context(os: Os, install: &Install, keys: Vec<VerifyingKey>) -> Context {
    Context {
        current: semver::Version::parse(OLD).unwrap(),
        target: support::target(os).into(),
        install: Ok(install.clone()),
        keys,
        installs: Ok(()),
    }
}

fn api(stub: &Stub) -> String {
    format!("{}/releases", stub.base)
}

/// The whole path, per system: staged by a check, installed when the
/// client closes, announced once at the next start.
#[test]
fn a_signed_update_is_downloaded_verified_staged_and_installed_on_exit() {
    for os in [Os::Linux, Os::MacOs, Os::Windows] {
        let stub = Stub::start();
        publish(&stub, os, Publish::Signed);
        let (base, install, _) = installed(os, OLD, "e2e");
        let mut checker = Checker::new(api(&stub), OLD);
        let outcome = checker.run(&context(os, &install, vec![test_key().verifying_key()]));
        assert_eq!(
            outcome,
            Outcome::Staged {
                version: NEWER.into(),
                page: format!("{}/page/v{NEWER}", stub.base),
            },
            "{os:?}"
        );
        // Asked as the owner's design says.
        let (path, headers) = stub.seen.lock().unwrap()[0].clone();
        assert_eq!(path, "/releases");
        assert_eq!(headers["user-agent"], format!("Baylee/{OLD}"));
        assert_eq!(headers["accept"], "application/vnd.github+json");
        // Staged, not yet installed: the running client is untouched.
        assert!(install.stage().join(NEW).join(os.program()).exists());
        assert!(snapshot(&base).values().any(|v| v.contains(OLD)));
        // A second check finds it staged and downloads nothing again.
        let before = stub.count();
        let again = checker.run(&context(os, &install, vec![test_key().verifying_key()]));
        assert!(matches!(again, Outcome::Staged { .. }));
        assert_eq!(stub.count(), before + 1, "{os:?}: only the list was asked");
        assert_eq!(
            stub.seen.lock().unwrap()[before]
                .1
                .get("if-none-match")
                .map(String::as_str),
            Some("\"list-1\"")
        );
        // "Exit".
        let applied = apply::apply(&install, OLD).unwrap();
        assert_eq!(applied.to, NEWER);
        let shot = snapshot(&base);
        let program = match os {
            Os::MacOs => "Baylee.app/Contents/MacOS/baylee-client",
            Os::Windows => "baylee-client.exe",
            Os::Linux => "baylee-client",
        };
        assert!(shot[program].ends_with(NEWER), "{os:?}: {}", shot[program]);
        assert_eq!(shot["my-notes.txt"], "file 644 mine");
        if os == Os::MacOs {
            assert_eq!(
                shot["Baylee.app/Contents/MacOS/assets"],
                "link ../Resources/assets"
            );
        }
        // "Next start".
        assert_eq!(apply::recover(&install), apply::Recovery::Nothing);
        assert_eq!(apply::take_applied(&install).unwrap().to, NEWER);
        assert_eq!(apply::take_applied(&install), None);
    }
}

/// Every refusal leaves the installation as it was, nothing staged, and
/// the archive deleted; the player is linked to the release instead.
fn refused(how: Publish, keys: Vec<VerifyingKey>) -> Manual {
    let os = Os::Linux;
    let stub = Stub::start();
    publish(&stub, os, how);
    let (base, install, old) = installed(os, OLD, "refused");
    let mut checker = Checker::new(api(&stub), OLD);
    let outcome = checker.run(&context(os, &install, keys));
    let Outcome::Available { version, page, why } = outcome else {
        panic!("not refused: {outcome:?}");
    };
    assert_eq!(version, NEWER);
    assert_eq!(page, format!("{}/page/v{NEWER}", stub.base));
    assert_eq!(snapshot(&base), expected(&old));
    assert_eq!(apply::staged(&install), None);
    let left: Vec<_> = std::fs::read_dir(install.stage())
        .map(|d| {
            d.flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    assert!(
        left.iter().all(|n| !n.ends_with(".tar.gz") && n != NEW),
        "left behind: {left:?}"
    );
    assert!(matches!(
        apply::apply(&install, OLD),
        Err(apply::ApplyError::NothingStaged)
    ));
    assert_eq!(snapshot(&base), expected(&old));
    why
}

#[test]
fn a_tampered_archive_is_refused_and_nothing_changes() {
    let why = refused(Publish::Tampered, vec![test_key().verifying_key()]);
    assert!(matches!(why, Manual::NotOurs(_)), "{why:?}");
}

/// The checksum is only a sanity check, but a failed one still refuses:
/// the signature is never even read.
#[test]
fn a_checksum_that_does_not_match_is_refused() {
    let why = refused(Publish::BadChecksum, vec![test_key().verifying_key()]);
    assert!(
        matches!(why, Manual::NotOurs(ref w) if w.contains("checksum")),
        "{why:?}"
    );
}

#[test]
fn an_archive_signed_by_another_key_is_refused() {
    let why = refused(Publish::SignedByAStranger, vec![test_key().verifying_key()]);
    assert!(matches!(why, Manual::NotOurs(_)), "{why:?}");
}

/// The client's own keys refuse the test key's signature: a release signed
/// with anything but the compiled key is never installed by a shipped
/// client.
#[test]
fn the_compiled_keys_refuse_the_test_key() {
    let why = refused(Publish::Signed, sign::trusted_keys());
    assert!(matches!(why, Manual::NotOurs(_)), "{why:?}");
}

#[test]
fn an_unsigned_release_is_only_linked_and_never_downloaded() {
    let why = refused(Publish::Unsigned, vec![test_key().verifying_key()]);
    assert_eq!(why, Manual::Unsigned);
}

/// A folder the player cannot write gets the link, and no download.
#[cfg(unix)]
#[test]
fn an_unwritable_installation_gets_the_link() {
    use std::os::unix::fs::PermissionsExt as _;
    let os = Os::Linux;
    let stub = Stub::start();
    publish(&stub, os, Publish::Signed);
    let (base, install, _) = installed(os, OLD, "unwritable-e2e");
    std::fs::set_permissions(&base, std::fs::Permissions::from_mode(0o555)).unwrap();
    if std::fs::write(base.join("probe"), "").is_ok() {
        let _ = std::fs::remove_file(base.join("probe"));
        std::fs::set_permissions(&base, std::fs::Permissions::from_mode(0o755)).unwrap();
        return; // root
    }
    let outcome =
        Checker::new(api(&stub), OLD).run(&context(os, &install, vec![test_key().verifying_key()]));
    std::fs::set_permissions(&base, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(
        matches!(
            outcome,
            Outcome::Available {
                why: Manual::NotWritable(_),
                ..
            }
        ),
        "{outcome:?}"
    );
    assert_eq!(stub.paths(), vec!["/releases".to_owned()]);
}

/// A development build and a player who switched installing off are told,
/// and nothing is downloaded.
#[test]
fn a_dev_build_or_installing_off_checks_but_never_downloads() {
    for manual in [Manual::DevBuild, Manual::Off] {
        let os = Os::Linux;
        let stub = Stub::start();
        publish(&stub, os, Publish::Signed);
        let (_, install, _) = installed(os, OLD, "dev");
        let mut context = context(os, &install, vec![test_key().verifying_key()]);
        context.installs = Err(manual.clone());
        let outcome = Checker::new(api(&stub), OLD).run(&context);
        assert!(
            matches!(&outcome, Outcome::Available { why, .. } if *why == manual),
            "{outcome:?}"
        );
        assert_eq!(stub.paths(), vec!["/releases".to_owned()]);
    }
}

#[test]
fn an_up_to_date_client_downloads_nothing() {
    let os = Os::Linux;
    let stub = Stub::start();
    publish(&stub, os, Publish::Signed);
    let (_, install, _) = installed(os, OLD, "uptodate");
    let mut context = context(os, &install, vec![test_key().verifying_key()]);
    context.current = semver::Version::parse(NEWER).unwrap();
    assert_eq!(
        Checker::new(api(&stub), NEWER).run(&context),
        Outcome::UpToDate
    );
    assert_eq!(stub.count(), 1);
}

/// GitHub's spent limit is waited out: the next check makes no request.
#[test]
fn a_spent_rate_limit_is_waited_out() {
    let stub = Stub::start();
    stub.route(
        "/releases",
        Answer {
            status: 403,
            headers: vec![
                ("x-ratelimit-remaining".into(), "0".into()),
                ("x-ratelimit-reset".into(), "99999999999".into()),
            ],
            body: br#"{"message":"API rate limit exceeded"}"#.to_vec(),
        },
    );
    let mut checker = Checker::new(api(&stub), OLD);
    assert!(matches!(
        checker.releases(),
        Err(baylee_update::check::CheckError::RateLimited(_))
    ));
    assert_eq!(stub.count(), 1);
    assert!(checker.releases().is_err());
    assert!(checker.releases().is_err());
    assert_eq!(stub.count(), 1, "asked again while the limit was spent");
}

/// Waits until `stub` has seen `n` requests, or a few seconds.
fn wait_for(stub: &Stub, n: usize) {
    for _ in 0..200 {
        if stub.count() >= n {
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// With automatic checks off, no request at all: not at start, not when
/// the period comes round (here every 30 ms). Asked by the player, one.
#[test]
fn with_automatic_checks_off_no_request_is_made() {
    let os = Os::Linux;
    let stub = Stub::start();
    publish(&stub, os, Publish::Signed);
    let (_, install, _) = installed(os, OLD, "off");
    let service = Service::start(
        api(&stub),
        context(os, &install, vec![test_key().verifying_key()]),
        Settings {
            check: false,
            install: true,
        },
        false,
        Duration::from_millis(30),
    )
    .unwrap();
    std::thread::sleep(Duration::from_millis(400));
    assert_eq!(stub.count(), 0, "a request with automatic checks off");
    assert!(service.outcomes.try_recv().is_err());
    // The same service, asked: the stub was there all along.
    service.send(Command::CheckNow);
    let outcome = service
        .outcomes
        .recv_timeout(Duration::from_secs(20))
        .unwrap();
    assert!(matches!(outcome, Outcome::Staged { .. }), "{outcome:?}");
    assert!(stub.count() >= 1);
}

/// On: at start, and again every period, the second time with the `ETag`.
#[test]
fn with_automatic_checks_on_it_asks_at_start_and_every_period() {
    let os = Os::Linux;
    let stub = Stub::start();
    publish(&stub, os, Publish::Unsigned);
    let (_, install, _) = installed(os, OLD, "on");
    let service = Service::start(
        api(&stub),
        context(os, &install, vec![test_key().verifying_key()]),
        Settings {
            check: true,
            install: true,
        },
        false,
        Duration::from_millis(100),
    )
    .unwrap();
    wait_for(&stub, 2);
    let seen = stub.seen.lock().unwrap().clone();
    assert!(seen.len() >= 2, "{seen:?}");
    assert_eq!(seen[0].1.get("if-none-match"), None);
    assert_eq!(
        seen[1].1.get("if-none-match").map(String::as_str),
        Some("\"list-1\"")
    );
    // Switched off while running: the period no longer asks.
    service.send(Command::Settings(Settings {
        check: false,
        install: true,
    }));
    std::thread::sleep(Duration::from_millis(150));
    let settled = stub.count();
    std::thread::sleep(Duration::from_millis(400));
    assert_eq!(
        stub.count(),
        settled,
        "asked again after being switched off"
    );
    drop(service);
}

#[test]
fn authentic_signatures_cannot_relabel_a_version_or_architecture() {
    for how in [Publish::WrongVersion, Publish::WrongArchitecture] {
        let why = refused(how, vec![test_key().verifying_key()]);
        assert!(
            matches!(why, Manual::Download(ref reason) if reason.contains("version/target")),
            "{why:?}"
        );
    }
}
