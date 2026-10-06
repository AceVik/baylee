//! The start and the exit on a real folder, and what an outcome shows.

use super::*;
use baylee_update::apply::Staged;
use baylee_update::plan::NEW;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::AtomicU32;

fn updater(install: &Install, allowed: bool) -> Updater {
    Updater {
        service: Mutex::new(None),
        install: Some(install.clone()),
        version: "0.1.0-beta.2".into(),
        dev: false,
        allowed: AtomicBool::new(allowed),
    }
}

/// What winit does on macOS's "Quit" (`bevy_winit`'s `exiting`): it clears
/// the world, and that alone has to install the update.
#[test]
fn clearing_the_world_as_quit_does_installs_the_update() {
    let (base, install) = installed();
    stage(&install);
    let mut world = World::new();
    world.insert_resource(updater(&install, true));
    world.clear_all();
    assert_eq!(program(&base), "new");
    let _ = fs::remove_dir_all(base);
}

#[test]
fn switching_installing_off_is_what_the_exit_obeys() {
    let (base, install) = installed();
    stage(&install);
    let mut app = App::new();
    app.add_message::<UpdateRequest>()
        .insert_resource(updater(&install, true))
        .add_systems(Update, forward);
    app.world_mut()
        .write_message(UpdateRequest::Prefs(UpdatePrefs {
            check: true,
            install: false,
        }));
    app.update();
    drop(app);
    assert_eq!(program(&base), "old");
    assert!(apply::staged(&install).is_some());
    let _ = fs::remove_dir_all(base);
}

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
    fs::write(base.join("baylee-runtime"), "old").expect("the old program");
    let install = Install {
        state: None,
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
    fs::write(new.join("baylee-runtime"), "new").expect("the new program");
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
    let install = Install {
        state: None,
        os: Os::Linux,
        base: base.to_path_buf(),
        program: "baylee-client".into(),
    };
    fs::read_to_string(launch::selected(&install).unwrap()).expect("a program")
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
    assert!(
        install.stage().join("current.json").exists(),
        "the launcher retains its pointer"
    );
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

fn seen(writable: bool, blocked: Option<Blocked>, translocated: bool) -> Seen {
    Seen {
        writable,
        blocked,
        translocated,
    }
}

/// Where the launcher's word becomes the face's: an app that installs in
/// place offers nothing; a read-only folder names itself and offers the
/// move (both destinations only where `/Applications` is writable); a
/// translocated app that installs anyway still offers the move.
#[test]
fn the_place_follows_what_the_launcher_said() {
    let fine = place_of(Some(&seen(true, None, false)), true, || unreachable!());
    assert_eq!(fine, UpdatePlace::default());

    let read_only = Some(Blocked::ReadOnly {
        folder: "/Applications".into(),
        error: "Permission denied (os error 13)".into(),
    });
    let place = place_of(Some(&seen(false, read_only.clone(), false)), true, || false);
    assert_eq!(
        place.read_only,
        Some((
            "/Applications".into(),
            "Permission denied (os error 13)".into()
        ))
    );
    assert_eq!(place.moves, [MoveTo::Home]);
    let place = place_of(Some(&seen(false, read_only, false)), true, || true);
    assert_eq!(place.moves, [MoveTo::Home, MoveTo::System]);

    let translocated = place_of(Some(&seen(true, None, true)), true, || false);
    assert!(translocated.translocated);
    assert_eq!(translocated.moves, [MoveTo::Home]);

    let unknown = place_of(
        Some(&seen(false, Some(Blocked::Translocated), true)),
        true,
        || false,
    );
    assert!(unknown.translocated);
    assert_eq!(unknown.moves, [MoveTo::Home]);

    // A launcher that predates the reasons: still the move, no words.
    let old = place_of(Some(&seen(false, None, false)), true, || false);
    assert_eq!(old.read_only, None);
    assert_eq!(old.moves, [MoveTo::Home]);

    // Not a bundle (Linux, Windows, `cargo run`), or no launcher: no move.
    let place = place_of(Some(&seen(false, None, true)), false, || true);
    assert!(place.moves.is_empty());
    assert_eq!(place_of(None, true, || true), UpdatePlace::default());
}

/// The move copies under the original's name, so a renamed app keeps its
/// name, and says why when there is nothing to copy.
#[cfg(unix)]
#[test]
fn the_move_keeps_the_originals_name() {
    let root = std::env::temp_dir().join(format!("baylee-client-move-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let source = root.join("mount/Baylee.app");
    fs::create_dir_all(source.join("Contents/MacOS")).unwrap();
    let mover = Mover {
        trash: Box::new(FakeTrash::default()),
        source: Some(source.clone()),
        original: Some(root.join("Downloads/Baylee Beta.app")),
        home: Some(root.join("home")),
    };
    if cfg!(target_os = "macos") {
        // An unsigned bundle is refused by the signature check, and nothing
        // is left behind; the signed path is `relocate`'s own test.
        let err = mover.copy(MoveTo::Home).unwrap_err();
        assert!(err.contains("signature"), "{err}");
        assert_eq!(
            fs::read_dir(root.join("home/Applications"))
                .unwrap()
                .count(),
            0
        );
    } else {
        let to = mover.copy(MoveTo::Home).unwrap();
        assert_eq!(to, root.join("home/Applications/Baylee Beta.app"));
    }
    let nothing = Mover {
        source: None,
        ..mover
    };
    assert!(nothing.copy(MoveTo::Home).is_err());
    fs::remove_dir_all(root).unwrap();
}

/// The Trash, faked: records what it was given, or refuses.
#[derive(Clone, Default)]
struct FakeTrash {
    taken: std::sync::Arc<Mutex<Vec<PathBuf>>>,
    refuse: bool,
}

impl Trash for FakeTrash {
    fn trash(&self, item: &Path) -> std::io::Result<PathBuf> {
        if self.refuse {
            return Err(std::io::Error::other("the volume has no Trash"));
        }
        self.taken.lock().unwrap().push(item.to_path_buf());
        Ok(Path::new("/Trash").join(item.file_name().unwrap()))
    }
}

fn mover_app(trash: FakeTrash) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_message::<bevy::picking::events::Pointer<bevy::picking::events::Click>>()
        .add_message::<AppExit>()
        .add_plugins(UpdatePlugin)
        .insert_resource(Mover {
            trash: Box::new(trash),
            source: None,
            original: None,
            home: None,
        })
        .add_systems(Update, relocate_on_request);
    app
}

/// "Move old copy to Trash" hands the old copy to the Trash and stops
/// asking; a refusal is said, and the question stays.
#[test]
fn the_old_copy_goes_to_the_trash_or_says_why_not() {
    let trash = FakeTrash::default();
    let mut app = mover_app(trash.clone());
    let moved = Some(("/new/Baylee.app".to_owned(), "/old/Baylee.app".to_owned()));
    app.world_mut().resource_mut::<UpdatePlace>().moved = moved.clone();
    app.world_mut().write_message(UpdateRequest::TrashOld);
    app.update();
    assert_eq!(
        *trash.taken.lock().unwrap(),
        [PathBuf::from("/old/Baylee.app")]
    );
    assert_eq!(app.world().resource::<UpdatePlace>().moved, None);

    let mut app = mover_app(FakeTrash {
        refuse: true,
        ..FakeTrash::default()
    });
    app.world_mut().resource_mut::<UpdatePlace>().moved = moved.clone();
    app.world_mut().write_message(UpdateRequest::TrashOld);
    app.update();
    let place = app.world().resource::<UpdatePlace>();
    assert_eq!(place.moved, moved, "still asked");
    assert_eq!(
        place.failed.as_deref(),
        Some(
            "It could not be moved to the Trash (the volume has no Trash); drag it there yourself if you like."
        )
    );
}

/// The buttons' requests reach the handler: a move that cannot copy says
/// why in the place and does not quit; keeping the old copy stops asking.
#[test]
fn a_failed_move_is_said_and_does_not_quit() {
    let mut app = mover_app(FakeTrash::default());
    app.world_mut()
        .write_message(UpdateRequest::Move(MoveTo::Home));
    app.update();
    let failed = app.world().resource::<UpdatePlace>().failed.clone();
    assert_eq!(
        failed.as_deref(),
        Some("Baylee could not be moved: Baylee is not running from an app bundle")
    );
    let exits = app.world().resource::<Messages<AppExit>>();
    assert!(exits.is_empty(), "a failed move never quits");

    app.world_mut().resource_mut::<UpdatePlace>().moved =
        Some(("/new/Baylee.app".into(), "/old/Baylee.app".into()));
    app.world_mut().write_message(UpdateRequest::KeepOld);
    app.update();
    let place = app.world().resource::<UpdatePlace>();
    assert_eq!(place.moved, None);
    assert_eq!(place.failed, None);
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
