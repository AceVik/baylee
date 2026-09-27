//! The start and the exit on a real folder, and what an outcome shows.

use super::*;
use baylee_update::apply::Staged;
use baylee_update::plan::{NEW, STAGE};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

/// A fresh folder with an "installed" Linux client in it (the plan is the
/// same renames on every system; the per-system trees are
/// `baylee-update`'s tests).
fn installed() -> (PathBuf, Install) {
    static N: AtomicU32 = AtomicU32::new(0);
    let base = std::env::temp_dir().join(format!(
        "baylee-client-update-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).expect("a scratch folder");
    fs::write(base.join("baylee-client"), "old").expect("the old program");
    let install = Install {
        os: Os::Linux,
        base: base.clone(),
        program: "baylee-client".into(),
    };
    (base, install)
}

/// Puts version `beta.3` in the stage as a finished check would.
fn stage(install: &Install) {
    let new = install.stage().join(NEW);
    fs::create_dir_all(&new).expect("the stage");
    fs::write(new.join("baylee-client"), "new").expect("the new program");
    apply::mark_staged(
        install,
        &Staged {
            version: "0.1.0-beta.3".into(),
            tag: "v0.1.0-beta.3".into(),
            page: "https://github.com/AceVik/baylee/releases/tag/v0.1.0-beta.3".into(),
            asset: "baylee-0.1.0-beta.3-x86_64-unknown-linux-gnu.tar.gz".into(),
            size: 3,
        },
    )
    .expect("the record");
}

fn build(install: &Install, dev: bool) -> Build {
    Build {
        version: Version::parse("0.1.0-beta.2").expect("a version"),
        target: "x86_64-unknown-linux-gnu".into(),
        api: "http://127.0.0.1:9/".into(),
        keys: Vec::new(),
        dev,
        install: Ok(install.clone()),
    }
}

fn program(base: &std::path::Path) -> String {
    fs::read_to_string(base.join("baylee-client")).expect("a program")
}

#[test]
fn a_test_build_is_a_development_build() {
    assert!(!is_release_build());
}

#[test]
fn the_exit_installs_and_the_next_start_says_so_once() {
    let (base, install) = installed();
    stage(&install);
    assert_eq!(
        on_start(&build(&install, false)).shown,
        Some(Shown::Ready {
            version: "0.1.0-beta.3".into(),
            page: "https://github.com/AceVik/baylee/releases/tag/v0.1.0-beta.3".into(),
        }),
        "a staged update is announced before any check"
    );

    let said = exit_with(&install, "0.1.0-beta.2", true, false).expect("something staged");
    assert!(said.starts_with("installed 0.1.0-beta.3"), "{said}");
    assert_eq!(program(&base), "new");

    let first = on_start(&build(&install, false));
    assert_eq!(
        first.shown,
        Some(Shown::Updated {
            version: "0.1.0-beta.3".into(),
            page: Some("https://github.com/AceVik/baylee/releases/tag/v0.1.0-beta.3".into()),
        })
    );
    assert_eq!(on_start(&build(&install, false)).shown, None, "only once");
    assert!(!base.join(STAGE).exists(), "nothing of ours is left");
    assert!(
        !base.join("baylee-client.old").exists(),
        "the old one is gone"
    );
    let _ = fs::remove_dir_all(base);
}

#[test]
fn a_development_build_never_installs() {
    let (base, install) = installed();
    stage(&install);
    let said = exit_with(&install, "0.1.0-beta.2", true, true).expect("something staged");
    assert!(said.contains("development build"), "{said}");
    assert_eq!(program(&base), "old");
    assert!(
        matches!(
            on_start(&build(&install, true)).shown,
            Some(Shown::Available {
                why: Why::DevBuild,
                ..
            })
        ),
        "and says so"
    );
    let _ = fs::remove_dir_all(base);
}

#[test]
fn installing_off_leaves_the_update_staged() {
    let (base, install) = installed();
    stage(&install);
    let said = exit_with(&install, "0.1.0-beta.2", false, false).expect("something staged");
    assert!(said.contains("off"), "{said}");
    assert_eq!(program(&base), "old");
    assert!(apply::staged(&install).is_some());
    let _ = fs::remove_dir_all(base);
}

#[test]
fn nothing_staged_is_nothing_to_do() {
    let (base, install) = installed();
    assert_eq!(exit_with(&install, "0.1.0-beta.2", true, false), None);
    assert_eq!(on_start(&build(&install, false)), UpdateNotice::default());
    assert_eq!(program(&base), "old");
    let _ = fs::remove_dir_all(base);
}

#[test]
fn an_outcome_reaches_the_notice_and_answers_a_check_the_player_asked_for() {
    let mut notice = UpdateNotice {
        asked: Some(Asked::Checking),
        ..UpdateNotice::default()
    };
    take(&mut notice, Outcome::UpToDate);
    assert_eq!(notice.asked, Some(Asked::UpToDate));
    assert_eq!(notice.shown, None);

    notice.asked = Some(Asked::Checking);
    take(&mut notice, Outcome::Unknown("offline".into()));
    assert_eq!(notice.asked, Some(Asked::Failed));

    notice.asked = None;
    notice.hidden = true;
    take(
        &mut notice,
        Outcome::Available {
            version: "0.1.0-beta.3".into(),
            page: "p".into(),
            why: Manual::NotOurs("stranger".into()),
        },
    );
    assert_eq!(
        notice.shown,
        Some(Shown::Available {
            version: "0.1.0-beta.3".into(),
            page: "p".into(),
            why: Why::NotVerified,
        })
    );
    assert!(!notice.hidden, "news shows again");
    assert_eq!(notice.asked, None, "a scheduled check answers nobody");

    notice.hidden = true;
    take(
        &mut notice,
        Outcome::Available {
            version: "0.1.0-beta.3".into(),
            page: "p".into(),
            why: Manual::NotOurs("stranger".into()),
        },
    );
    assert!(notice.hidden, "the same news stays hidden");

    notice.asked = Some(Asked::Checking);
    take(
        &mut notice,
        Outcome::Staged {
            version: "0.1.0-beta.3".into(),
            page: "p".into(),
        },
    );
    assert_eq!(notice.asked, Some(Asked::Found));
    assert!(matches!(notice.shown, Some(Shown::Ready { .. })));
}

#[test]
fn every_reason_is_worded() {
    for (manual, why) in [
        (Manual::Off, Why::Off),
        (Manual::DevBuild, Why::DevBuild),
        (Manual::NotWritable("ro".into()), Why::Folder),
        (Manual::Unplaceable(Unplaceable::Translocated), Why::MoveApp),
        (Manual::Unplaceable(Unplaceable::NotABundle), Why::Other),
        (Manual::Unsigned, Why::NotVerified),
        (Manual::NotOurs("x".into()), Why::NotVerified),
        (Manual::NoArchive, Why::Other),
        (Manual::FailedBefore("x".into()), Why::Other),
        (Manual::Download("x".into()), Why::Other),
    ] {
        assert_eq!(why_of(&manual), why, "{manual:?}");
    }
}
