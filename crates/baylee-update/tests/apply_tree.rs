//! Applying an update with real renames, in a temporary directory laid out
//! as each system's installation is, on whatever machine runs the tests.
//! And a crash after every step: the next start always converges on a
//! client that starts.

mod support;

use baylee_update::apply::{self, ApplyError, Install, Journal, Recovery, Staged};
use baylee_update::plan::{NEW, Os, STAGE};
use support::{Tree, expected, installed, release_tree, snapshot, write_tree};

const OLD: &str = "0.1.0-beta.2";
const NEWER: &str = "0.1.0-beta.3";
/// Every system whose installation this host can lay out: the macOS bundle
/// holds a symlink, which Windows makes only in developer mode, so a Windows
/// host checks Windows and Linux and leaves the bundle to the Mac and CI.
fn systems() -> impl Iterator<Item = Os> {
    [Os::Windows, Os::MacOs, Os::Linux]
        .into_iter()
        .filter(|os| cfg!(unix) || *os != Os::MacOs)
}

/// Puts the newer release's tree where the updater stages it.
fn stage(install: &Install) -> Tree {
    let tree = release_tree(install.os, NEWER);
    write_tree(&install.stage().join(NEW), &tree);
    apply::mark_staged(
        install,
        &Staged {
            version: NEWER.into(),
            tag: format!("v{NEWER}"),
            page: "https://example.test/release".into(),
            asset: "archive".into(),
            size: 1,
        },
    )
    .unwrap();
    tree
}

/// What the installation should be once the update is in: every entry the
/// newer release brings replaced (on macOS, beside the bundle, only those
/// that were there), and the player's own file untouched.
fn after(os: Os, old: &Tree, new: &Tree) -> Tree {
    let top = |rel: &str| rel.split('/').next().unwrap().to_owned();
    let replaced: std::collections::BTreeSet<String> = new
        .keys()
        .map(|rel| top(rel))
        .filter(|name| {
            os != Os::MacOs || name == "Baylee.app" || old.keys().any(|r| top(r) == *name)
        })
        .collect();
    let mut tree: Tree = old
        .iter()
        .filter(|(rel, _)| !replaced.contains(&top(rel)))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    tree.extend(
        new.iter()
            .filter(|(rel, _)| replaced.contains(&top(rel)))
            .map(|(k, v)| (k.clone(), v.clone())),
    );
    tree
}

#[test]
fn each_system_is_replaced_by_its_new_tree() {
    for os in systems() {
        let (base, install, old) = installed(os, OLD, "each");
        let new = stage(&install);
        let applied = apply::apply(&install, OLD).unwrap();
        assert_eq!(applied.to, NEWER);
        assert_eq!(snapshot(&base), expected(&after(os, &old, &new)), "{os:?}");
        // The player's file, and none of the old release's own.
        assert!(base.join("my-notes.txt").exists());
        assert!(
            !snapshot(&base).keys().any(|k| k.contains("Gone.ttf")),
            "{os:?}"
        );
        // Said once at the next start, then never again.
        let told = apply::take_applied(&install).unwrap();
        assert_eq!((told.from.as_str(), told.to.as_str()), (OLD, NEWER));
        assert_eq!(apply::take_applied(&install), None);
        assert!(
            install.stage().join("mutation.lock").exists(),
            "{os:?}: the permanent lock must stay"
        );
    }
}

/// A bundle in `/Applications`: only the bundle is replaced, and the
/// licences the archive carries are not dropped beside it.
#[cfg(unix)] // a bundle holds a symlink (`systems`)
#[test]
fn macos_in_applications_touches_nothing_but_the_bundle() {
    let base = support::scratch("applications").join("Applications");
    let old: Tree = release_tree(Os::MacOs, OLD)
        .into_iter()
        .filter(|(rel, _)| rel.starts_with("Baylee.app/"))
        .collect();
    write_tree(&base, &old);
    // Through `write_tree`, which states the mode: a plain write takes the
    // umask's, 664 under Ubuntu's 002 for a user.
    let safari = support::Node::File(b"safari".to_vec(), 0o644);
    write_tree(
        &base,
        &Tree::from([("Safari.app/Contents/Info.plist".to_owned(), safari)]),
    );
    let install = Install::around(&support::exe(Os::MacOs, &base), Os::MacOs).unwrap();
    let new = stage(&install);
    apply::apply(&install, OLD).unwrap();
    let shot = snapshot(&base);
    assert!(!shot.contains_key("LICENSE"));
    assert!(!shot.contains_key("README.txt"));
    assert!(!shot.keys().any(|k| k.starts_with("baylee-client.dSYM")));
    assert_eq!(shot["Safari.app/Contents/Info.plist"], "file 644 safari");
    assert_eq!(
        shot["Baylee.app/Contents/MacOS/baylee-client"],
        expected(&new)["Baylee.app/Contents/MacOS/baylee-client"]
    );
    // The bundle's link came through as a link.
    assert_eq!(
        shot["Baylee.app/Contents/MacOS/assets"],
        "link ../Resources/assets"
    );
}

/// Windows cannot delete the program that is running the update, so it is
/// renamed to `baylee-client.old.exe`, and the next start deletes it. The
/// steps are watched one by one up to that point.
#[test]
fn the_running_windows_exe_is_set_aside_and_deleted_at_the_next_start() {
    let (base, install, _) = installed(Os::Windows, OLD, "windows-exe");
    stage(&install);
    let mut journal = Journal::begin(&install, OLD).unwrap();
    // Every step but the last: the running program is set aside, the new
    // one not yet in.
    for _ in 0..journal.steps.len() - 1 {
        assert!(journal.step(&install).unwrap());
    }
    assert!(!base.join("baylee-client.exe").exists());
    assert_eq!(
        std::fs::read_to_string(base.join("baylee-client.old.exe")).unwrap(),
        format!("windows program {OLD}")
    );
    assert!(journal.step(&install).unwrap());
    assert!(!journal.step(&install).unwrap());
    assert_eq!(
        std::fs::read_to_string(base.join("baylee-client.exe")).unwrap(),
        format!("windows program {NEWER}")
    );
    // The process ends here, before its own clean-up; the next start
    // finishes the bookkeeping. The old program, still running while the
    // update finished, could not be deleted then: it is there again.
    assert!(matches!(apply::recover(&install), Recovery::Finished(_)));
    std::fs::write(base.join("baylee-client.old.exe"), "was running").unwrap();
    assert_eq!(apply::take_applied(&install).unwrap().to, NEWER);
    assert!(!base.join("baylee-client.old.exe").exists());
}

/// A crash after any step, recorded in the journal or not yet, is finished
/// at the next start: the installation is then exactly the new one.
#[test]
fn a_crash_after_any_step_is_finished_at_the_next_start() {
    for os in systems() {
        let steps = {
            let (_, install, _) = installed(os, OLD, "count");
            stage(&install);
            Journal::begin(&install, OLD).unwrap().steps.len()
        };
        for done in 0..=steps {
            for unrecorded in [false, true] {
                if unrecorded && done == steps {
                    continue;
                }
                let (base, install, old) = installed(os, OLD, "crash");
                let new = stage(&install);
                let mut journal = Journal::begin(&install, OLD).unwrap();
                for _ in 0..done {
                    journal.step(&install).unwrap();
                }
                if unrecorded {
                    // The rename happened; the process died before the
                    // journal said so.
                    let step = &journal.steps[done];
                    let at = |rel: &[String]| {
                        if rel.first().is_some_and(|s| s == STAGE) {
                            rel[1..].iter().fold(install.stage(), |p, s| p.join(s))
                        } else {
                            rel.iter().fold(base.clone(), |p, s| p.join(s))
                        }
                    };
                    std::fs::rename(at(&step.from), at(&step.to)).unwrap();
                }
                let recovered = apply::recover(&install);
                assert!(
                    matches!(recovered, Recovery::Finished(ref a) if a.to == NEWER),
                    "{os:?} after {done} steps (unrecorded {unrecorded}): {recovered:?}"
                );
                assert_eq!(
                    snapshot(&base),
                    expected(&after(os, &old, &new)),
                    "{os:?} after {done} steps (unrecorded {unrecorded})"
                );
                assert!(apply::take_applied(&install).is_some());
            }
        }
    }
}

/// When the staged tree is lost as well (a disk cleaner, a player tidying
/// up), the update cannot be finished, and every done step is undone: the
/// old installation, exactly.
#[test]
fn a_crash_that_cannot_be_finished_is_rolled_back_to_the_old_client() {
    for os in systems() {
        let steps = {
            let (_, install, _) = installed(os, OLD, "count");
            stage(&install);
            Journal::begin(&install, OLD).unwrap().steps.len()
        };
        for done in 0..steps {
            let (base, install, old) = installed(os, OLD, "rollback");
            stage(&install);
            let mut journal = Journal::begin(&install, OLD).unwrap();
            for _ in 0..done {
                journal.step(&install).unwrap();
            }
            std::fs::remove_dir_all(install.stage().join(NEW)).unwrap();
            let recovered = apply::recover(&install);
            // A crash before any move-in whose source is gone would have
            // nothing left to finish with; one after the last would have
            // nothing left to do.
            assert_eq!(recovered, Recovery::RolledBack, "{os:?} after {done} steps");
            assert_eq!(snapshot(&base), expected(&old), "{os:?} after {done} steps");
            assert_eq!(apply::failed(&install).unwrap().version, NEWER);
            assert_eq!(apply::staged(&install), None);
            assert!(!install.stage().join("journal.json").exists());
        }
    }
}

/// A rename that fails half way undoes the ones before it, records the
/// version as failed, and leaves the old client exactly as it was. The
/// failure: the staged folder cannot be written, so nothing can be moved
/// out of it, while the old entries can still be set aside.
#[cfg(unix)]
#[test]
fn a_step_that_fails_undoes_the_ones_before_it() {
    use std::os::unix::fs::PermissionsExt as _;
    for os in systems() {
        let (base, install, old) = installed(os, OLD, "fails");
        stage(&install);
        let new = install.stage().join(NEW);
        std::fs::set_permissions(&new, std::fs::Permissions::from_mode(0o555)).unwrap();
        let result = apply::apply(&install, OLD);
        let _ = std::fs::set_permissions(&new, std::fs::Permissions::from_mode(0o755));
        if result.is_ok() {
            // Root renames anywhere; there is nothing to measure.
            return;
        }
        assert!(matches!(result, Err(ApplyError::Io(_))), "{os:?}");
        assert_eq!(snapshot(&base), expected(&old), "{os:?}");
        assert_eq!(apply::failed(&install).unwrap().version, NEWER);
        assert!(!install.stage().join("journal.json").exists());
        // Nothing is staged any more, so nothing is tried again.
        assert!(matches!(
            apply::apply(&install, OLD),
            Err(ApplyError::NothingStaged)
        ));
    }
}

/// An aside name taken by something the plan does not know is refused
/// before any rename.
#[test]
fn a_taken_aside_refuses_before_any_rename() {
    for os in systems() {
        let (base, install, old) = installed(os, OLD, "taken");
        stage(&install);
        let aside = base.join(os.aside(&install.program));
        std::fs::create_dir_all(&aside).unwrap();
        let err = apply::apply(&install, OLD).unwrap_err();
        assert!(matches!(err, ApplyError::Plan(_)), "{os:?}: {err}");
        std::fs::remove_dir_all(&aside).unwrap();
        assert_eq!(snapshot(&base), expected(&old), "{os:?}");
        assert_eq!(apply::failed(&install).unwrap().version, NEWER);
    }
}

/// Nothing staged, nothing done: a client closed with no update pending.
#[test]
fn nothing_staged_changes_nothing() {
    for os in systems() {
        let (base, install, old) = installed(os, OLD, "nothing");
        assert!(matches!(
            apply::apply(&install, OLD),
            Err(ApplyError::NothingStaged)
        ));
        assert_eq!(apply::recover(&install), Recovery::Nothing);
        assert_eq!(apply::take_applied(&install), None);
        assert_eq!(snapshot(&base), expected(&old));
    }
}

/// A folder the player cannot write is reported, not forced.
#[cfg(unix)]
#[test]
fn an_unwritable_folder_is_reported() {
    use std::os::unix::fs::PermissionsExt as _;
    let (base, install, _) = installed(Os::Linux, OLD, "unwritable");
    std::fs::set_permissions(&base, std::fs::Permissions::from_mode(0o555)).unwrap();
    let probe = std::fs::write(base.join("probe"), "");
    if probe.is_ok() {
        // Root writes anywhere; there is nothing to measure.
        let _ = std::fs::remove_file(base.join("probe"));
        std::fs::set_permissions(&base, std::fs::Permissions::from_mode(0o755)).unwrap();
        return;
    }
    assert!(install.writable().is_err());
    assert!(!install.stage().exists());
    std::fs::set_permissions(&base, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(install.writable().is_ok());
}

#[test]
fn rollback_failure_retains_journal_and_payload_until_a_retry_succeeds() {
    let (base, install, old) = installed(Os::Linux, OLD, "rollback-retry");
    stage(&install);
    let mut journal = Journal::begin(&install, OLD).unwrap();
    journal.step(&install).unwrap(); // LICENSE -> LICENSE.old
    journal.step(&install).unwrap(); // new/LICENSE -> LICENSE
    // Force rollback, then obstruct its first reverse rename.
    std::fs::remove_file(install.stage().join(NEW).join("baylee-client")).unwrap();
    let obstruction = install.stage().join(NEW).join("LICENSE");
    std::fs::write(&obstruction, "obstruction").unwrap();
    assert_eq!(apply::recover(&install), Recovery::Deferred);
    let path = install.stage().join("journal.json");
    let saved: Journal = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert!(saved.rolling_back);
    assert_eq!(saved.done, 2);
    assert!(install.stage().join(NEW).exists());
    assert!(base.join("LICENSE.old").exists());
    std::fs::remove_file(obstruction).unwrap();
    assert_eq!(apply::recover(&install), Recovery::RolledBack);
    assert!(!path.exists());
    assert_eq!(snapshot(&base), expected(&old));
}

#[test]
fn crash_after_reverse_rename_before_recording_it_resumes_rollback() {
    let (base, install, old) = installed(Os::Linux, OLD, "rollback-crash");
    stage(&install);
    let mut journal = Journal::begin(&install, OLD).unwrap();
    journal.step(&install).unwrap();
    journal.step(&install).unwrap();
    journal.rolling_back = true;
    std::fs::write(
        install.stage().join("journal.json"),
        serde_json::to_vec(&journal).unwrap(),
    )
    .unwrap();
    // The first reverse rename happened, its counter write did not.
    std::fs::rename(
        base.join("LICENSE"),
        install.stage().join(NEW).join("LICENSE"),
    )
    .unwrap();
    assert_eq!(apply::recover(&install), Recovery::RolledBack);
    assert_eq!(snapshot(&base), expected(&old));
}
