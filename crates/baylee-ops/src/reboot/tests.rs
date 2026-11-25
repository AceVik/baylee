use super::*;
use std::fs;

fn dir() -> tempfile::TempDir {
    tempfile::tempdir().expect("a folder for the test")
}

#[test]
fn a_hold_is_claimed_once_and_stays_claimed_by_the_same_operation() {
    let dir = dir();
    let hold = dir.path().join("deep/er/hold");
    assert!(claim_hold(&hold), "with its folder still missing");
    assert_eq!(fs::read_to_string(&hold).unwrap(), HOLD_OWNER);
    // The unit runs again every hour while a reboot is pending.
    assert!(claim_hold(&hold));
    assert_eq!(fs::read_to_string(&hold).unwrap(), HOLD_OWNER);
}

#[test]
fn a_hold_that_belongs_to_another_operation_is_neither_taken_nor_changed() {
    let dir = dir();
    let hold = dir.path().join("hold");
    for owner in [
        "",
        "deploy\n",
        // Close is not ours either.
        "baylee-reboot admission hold",
        "baylee-reboot admission hold\n\n",
    ] {
        fs::write(&hold, owner).unwrap();
        assert!(!claim_hold(&hold), "{owner:?}");
        assert_eq!(fs::read_to_string(&hold).unwrap(), owner);
    }
}

#[test]
fn a_hold_that_cannot_be_created_is_not_claimed() {
    let dir = dir();
    let file = dir.path().join("file");
    fs::write(&file, "in the way").unwrap();
    assert!(!claim_hold(&file.join("hold")));
    assert!(!claim_hold(dir.path()), "a folder is no hold");
}

#[test]
fn letting_go_removes_our_hold_and_the_drain_mark() {
    let dir = dir();
    let (hold, drained) = (dir.path().join("hold"), dir.path().join("drained"));
    assert!(claim_hold(&hold));
    fs::write(&drained, "x").unwrap();
    undrain(Some(&hold), &drained);
    assert!(!hold.exists());
    assert!(!drained.exists());
    // Nothing to let go of is no error.
    undrain(Some(&hold), &drained);
    undrain(None, &drained);
}

#[test]
fn letting_go_never_opens_a_door_another_operation_is_holding() {
    let dir = dir();
    let (hold, drained) = (dir.path().join("hold"), dir.path().join("drained"));
    fs::write(&hold, "deploy\n").unwrap();
    fs::write(&drained, "x").unwrap();
    undrain(Some(&hold), &drained);
    assert_eq!(fs::read_to_string(&hold).unwrap(), "deploy\n");
    assert!(!drained.exists(), "the drain mark is always ours");
}

#[test]
fn a_running_deploy_is_seen_by_its_lock_and_a_finished_one_is_not() {
    let dir = dir();
    let lock = dir.path().join("lock");
    assert!(matches!(hold(&lock), Ok(None)), "never deployed");
    fs::write(&lock, "").unwrap();
    let held = hold(&lock).expect("free").expect("a lock file");
    assert!(hold(&lock).is_err(), "taken");
    drop(held);
    assert!(matches!(hold(&lock), Ok(Some(_))), "free again");
}

#[test]
fn the_age_of_a_missing_file_is_nothing_and_of_an_old_one_its_seconds() {
    let dir = dir();
    let file = dir.path().join("required");
    assert_eq!(age_secs(&file), 0);
    let made = fs::File::create(&file).unwrap();
    assert!(age_secs(&file) < 60);
    let long_ago = std::time::SystemTime::now() - std::time::Duration::from_hours(4 * 24);
    made.set_modified(long_ago).unwrap();
    assert_eq!(age_secs(&file) / 86_400, 4);
    // A clock set back does not make a file infinitely old.
    let ahead = std::time::SystemTime::now() + std::time::Duration::from_hours(24);
    made.set_modified(ahead).unwrap();
    assert_eq!(age_secs(&file), 0);
}
