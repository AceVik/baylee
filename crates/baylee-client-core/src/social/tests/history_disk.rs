//! The chat's files (§16, "History"): lines appended only with a history
//! length, restored and pruned at start, a damaged record skipped and the
//! rest loaded, file names that don't name the partner, and the folder on a
//! real file system.

use baylee_protocol::social::Conv;

use super::fakegw::{Client, Gateway};
use crate::social::{ChatDisk, ChatPrefs, Effect, Intent, disk, history, vault};

const ALICE: &str = "af03";
const BOB: &str = "0012";
const CAROL: &str = "0c11";
const DM_AB: &str = "dm:0012:af03";
const DAY: u64 = 86_400_000;

fn world() -> Gateway {
    let mut gw = Gateway::default();
    for (tag, handle) in [
        (ALICE, "Alice#af03"),
        (BOB, "Bob#0012"),
        (CAROL, "Carol#0c11"),
    ] {
        gw.handles.insert(tag.into(), handle.into());
    }
    gw.befriend(ALICE, BOB);
    gw.befriend(CAROL, BOB);
    gw
}

fn prefs(history_days: u16) -> ChatPrefs {
    ChatPrefs {
        history_days,
        ..ChatPrefs::default()
    }
}

/// Alice and Bob, friends, both running with `prefs` at the time `now`.
fn pair(prefs: &ChatPrefs, now: u64) -> (Gateway, Client, Client) {
    let mut gw = world();
    let mut alice = Client::with_prefs(&mut gw, ALICE, "Alice#af03", 1, prefs.clone());
    let mut bob = Client::with_prefs(&mut gw, BOB, "Bob#0012", 2, prefs.clone());
    alice.now = now;
    bob.now = now;
    (gw, alice, bob)
}

fn appended(client: &Client) -> usize {
    client
        .log
        .iter()
        .filter(|e| matches!(e, Effect::AppendHistory { .. }))
        .count()
}

fn rewrites(effects: &[Effect]) -> Vec<(&String, &Vec<u8>)> {
    effects
        .iter()
        .filter_map(|e| match e {
            Effect::RewriteHistory { file, bytes } => Some((file, bytes)),
            _ => None,
        })
        .collect()
}

fn history_files(client: &Client) -> Vec<String> {
    client
        .disk
        .files
        .keys()
        .filter(|p| p.starts_with("history/"))
        .cloned()
        .collect()
}

fn vault_key(client: &Client) -> [u8; 32] {
    *vault::decode_secrets(client.secrets.as_deref().unwrap())
        .unwrap()
        .vault_key()
}

fn conv(text: &str) -> Conv {
    text.parse().unwrap()
}

// ---------------------------------------------------------------------------
// Appending.
// ---------------------------------------------------------------------------

#[test]
fn lines_are_appended_to_the_history_when_it_is_on() {
    let (mut gw, mut alice, mut bob) = pair(&prefs(30), 1_000);
    alice.say(&mut gw, "/w Bob one");
    alice.say(&mut gw, "/w Bob two");
    bob.deliver(&mut gw);
    bob.say(&mut gw, "/w Alice three");
    alice.deliver(&mut gw);

    // Each side appends a record for each of the three lines it shows.
    assert_eq!(appended(&alice), 3);
    assert_eq!(appended(&bob), 3);
    for client in [&alice, &bob] {
        let files = history_files(client);
        assert_eq!(files.len(), 1, "one log for the one conversation");
        let key = vault_key(client);
        assert_eq!(files[0], history::file_path(&key, &conv(DM_AB)));
        let bytes = client.disk.read(&files[0]).unwrap();
        let loaded = history::read(&key, &history::file_name(&key, &conv(DM_AB)), &bytes);
        assert_eq!(loaded.skipped, 0);
        assert_eq!(loaded.records.len(), 3);
    }
}

#[test]
fn with_zero_history_days_nothing_is_appended_or_written_to_disk() {
    let (mut gw, mut alice, mut bob) = pair(&prefs(0), 1_000);
    alice.say(&mut gw, "/w Bob one");
    bob.deliver(&mut gw);
    bob.say(&mut gw, "/w Alice two");
    alice.deliver(&mut gw);

    assert_eq!(alice.texts(DM_AB), ["one", "two"], "the lines are shown");
    assert_eq!(bob.texts(DM_AB), ["one", "two"]);
    for client in [&alice, &bob] {
        assert_eq!(appended(client), 0, "no AppendHistory");
        assert!(history_files(client).is_empty(), "no log on the disk");
        assert!(
            client.disk.files.contains_key(disk::STATE_FILE),
            "the state file is not history"
        );
    }
}

#[test]
fn turning_the_history_off_stops_the_appending_from_then_on() {
    let (mut gw, mut alice, mut bob) = pair(&prefs(30), 1_000);
    alice.say(&mut gw, "/w Bob one");
    bob.deliver(&mut gw);
    assert_eq!(appended(&bob), 1);
    let file = history_files(&bob).remove(0);
    let size = bob.disk.read(&file).unwrap().len();

    bob.act(&mut gw, Intent::SetPrefs(prefs(0)));
    alice.say(&mut gw, "/w Bob two");
    bob.deliver(&mut gw);
    assert_eq!(bob.texts(DM_AB), ["one", "two"]);
    assert_eq!(appended(&bob), 1, "nothing more was appended");
    assert_eq!(bob.disk.read(&file).unwrap().len(), size);
}

// ---------------------------------------------------------------------------
// Restoring and pruning.
// ---------------------------------------------------------------------------

/// Three lines at the gateway's times 10, 40 and 59 days, read at day 60.
fn three_ages(prefs: &ChatPrefs) -> (Gateway, Client, Client) {
    let (mut gw, mut alice, mut bob) = pair(prefs, 60 * DAY);
    gw.clock = 10 * DAY;
    alice.say(&mut gw, "/w Bob ancient");
    gw.clock = 40 * DAY;
    alice.say(&mut gw, "/w Bob older");
    gw.clock = 59 * DAY;
    alice.say(&mut gw, "/w Bob recent");
    bob.deliver(&mut gw);
    assert_eq!(bob.texts(DM_AB), ["ancient", "older", "recent"]);
    (gw, alice, bob)
}

#[test]
fn a_restart_brings_the_lines_back_and_prunes_what_is_older_than_the_history_days() {
    let (mut gw, _alice, mut bob) = three_ages(&prefs(30));
    let key = vault_key(&bob);

    bob.restart(&mut gw);
    assert!(bob.texts(DM_AB).is_empty(), "nothing before restore");
    let effects = bob.restore(&mut gw);

    assert_eq!(
        bob.texts(DM_AB),
        ["older", "recent"],
        "30 days back from day 60"
    );
    let rewritten = rewrites(&effects);
    assert_eq!(
        rewritten.len(),
        1,
        "the log is rewritten without the old record"
    );
    let (file, bytes) = rewritten[0];
    let name = history::file_name(&key, &conv(DM_AB));
    assert_eq!(*file, history::file_path(&key, &conv(DM_AB)));
    let loaded = history::read(&key, &name, bytes);
    assert_eq!(loaded.skipped, 0);
    let texts: Vec<&str> = loaded
        .records
        .iter()
        .map(|r| r.body.text.as_str())
        .collect();
    assert_eq!(texts, ["older", "recent"]);
    assert_eq!(bob.disk.read(file).as_ref(), Some(bytes), "and written");

    // Pruned already: a second start has nothing to rewrite.
    bob.restart(&mut gw);
    let effects = bob.restore(&mut gw);
    assert!(rewrites(&effects).is_empty());
    assert_eq!(bob.texts(DM_AB), ["older", "recent"]);
}

#[test]
fn a_shorter_history_prunes_further_at_the_next_start() {
    let (mut gw, _alice, mut bob) = three_ages(&prefs(30));
    bob.prefs = prefs(7);
    bob.restart(&mut gw);
    bob.restore(&mut gw);
    assert_eq!(bob.texts(DM_AB), ["recent"], "7 days back from day 60");
}

#[test]
fn a_log_with_nothing_left_after_pruning_is_removed() {
    let (mut gw, _alice, mut bob) = three_ages(&prefs(30));
    let file = history_files(&bob).remove(0);
    bob.now = 400 * DAY;
    bob.restart(&mut gw);
    let effects = bob.restore(&mut gw);
    let rewritten = rewrites(&effects);
    assert_eq!(rewritten.len(), 1);
    assert!(rewritten[0].1.is_empty(), "an empty rewrite means remove");
    assert!(bob.disk.read(&file).is_none(), "the file is gone");
    assert!(bob.texts(DM_AB).is_empty());
}

#[test]
fn restored_lines_keep_who_wrote_them_and_whether_they_are_mine() {
    let (mut gw, mut alice, mut bob) = pair(&prefs(30), 1_000);
    alice.say(&mut gw, "/w Bob hello [@Bob#0012]");
    bob.deliver(&mut gw);
    bob.say(&mut gw, "/w Alice hi");
    alice.deliver(&mut gw);

    bob.restart(&mut gw);
    bob.restore(&mut gw);
    let lines = bob.message_lines(DM_AB);
    assert_eq!(lines.len(), 2);
    let crate::social::LineKind::Message { from, mine, body } = &lines[0].kind else {
        panic!("a message");
    };
    assert_eq!((from.handle.as_str(), *mine), ("Alice#af03", false));
    assert_eq!(body.text, "hello [@Bob#0012]");
    assert!(lines[0].mentions_me, "the mention survived");
    let crate::social::LineKind::Message { mine, .. } = &lines[1].kind else {
        panic!("a message");
    };
    assert!(*mine);
    assert_eq!(
        bob.conv(DM_AB).unwrap().unread,
        0,
        "restored lines are read"
    );
}

// ---------------------------------------------------------------------------
// Damage.
// ---------------------------------------------------------------------------

/// The byte range of each framed record in a log.
fn record_ranges(bytes: &[u8]) -> Vec<std::ops::Range<usize>> {
    let mut at = 0;
    let mut out = Vec::new();
    while at + 4 <= bytes.len() {
        let len =
            u32::from_be_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]) as usize;
        out.push(at + 4..at + 4 + len);
        at += 4 + len;
    }
    out
}

/// Bob with a three-line log of equal-sized records; returns the log's path.
fn bob_with_three_records() -> (Gateway, Client, String) {
    let (mut gw, mut alice, mut bob) = pair(&prefs(30), 1_000);
    for text in ["msg-1", "msg-2", "msg-3"] {
        alice.say(&mut gw, &format!("/w Bob {text}"));
    }
    bob.deliver(&mut gw);
    let file = history_files(&bob).remove(0);
    assert_eq!(record_ranges(&bob.disk.read(&file).unwrap()).len(), 3);
    (gw, bob, file)
}

#[test]
fn a_corrupted_record_is_skipped_and_counted_and_the_rest_load() {
    let (mut gw, mut bob, file) = bob_with_three_records();
    let key = vault_key(&bob);
    let mut bytes = bob.disk.read(&file).unwrap();
    let middle = record_ranges(&bytes)[1].clone();
    bytes[middle.start + middle.len() / 2] ^= 0xff;
    bob.disk.write(&file, &bytes).unwrap();

    let loaded = history::read(&key, &history::file_name(&key, &conv(DM_AB)), &bytes);
    assert_eq!(loaded.skipped, 1, "counted");
    let texts: Vec<&str> = loaded
        .records
        .iter()
        .map(|r| r.body.text.as_str())
        .collect();
    assert_eq!(texts, ["msg-1", "msg-3"]);

    // The machine takes the same view, and drops the damage from the file.
    bob.restart(&mut gw);
    let effects = bob.restore(&mut gw);
    assert_eq!(bob.texts(DM_AB), ["msg-1", "msg-3"]);
    assert_eq!(rewrites(&effects).len(), 1, "rewritten without the damage");
    let healed = bob.disk.read(&file).unwrap();
    let again = history::read(&key, &history::file_name(&key, &conv(DM_AB)), &healed);
    assert_eq!((again.skipped, again.records.len()), (0, 2));
}

#[test]
fn a_torn_last_write_loses_only_that_record() {
    let (mut gw, mut bob, file) = bob_with_three_records();
    let mut bytes = bob.disk.read(&file).unwrap();
    bytes.truncate(bytes.len() - 7);
    bob.disk.write(&file, &bytes).unwrap();
    bob.restart(&mut gw);
    bob.restore(&mut gw);
    assert_eq!(bob.texts(DM_AB), ["msg-1", "msg-2"]);
}

#[test]
fn a_log_does_not_open_under_another_files_name_or_another_devices_key() {
    let (_gw, bob, file) = bob_with_three_records();
    let key = vault_key(&bob);
    let bytes = bob.disk.read(&file).unwrap();
    let right = history::file_name(&key, &conv(DM_AB));
    assert_eq!(history::read(&key, &right, &bytes).records.len(), 3);

    // A record moved into another conversation's file fails there.
    let other = history::file_name(&key, &conv("dm:0012:0c11"));
    let moved = history::read(&key, &other, &bytes);
    assert_eq!((moved.records.len(), moved.skipped), (0, 3));
    // And another device's vault key opens none of it.
    let mut stranger = key;
    stranger[0] ^= 1;
    let foreign = history::read(&stranger, &right, &bytes);
    assert_eq!((foreign.records.len(), foreign.skipped), (0, 3));
    // The state file's records don't pass for history either.
    let state = bob.disk.read(disk::STATE_FILE).unwrap();
    assert_eq!(
        history::read(&key, disk::STATE_FILE, &state).records.len(),
        0
    );
}

// ---------------------------------------------------------------------------
// File names.
// ---------------------------------------------------------------------------

#[test]
fn history_file_names_do_not_name_the_partner_and_differ_between_two_devices_of_one_account() {
    let mut gw = world();
    let mut bob1 = Client::new(&mut gw, BOB, "Bob#0012", 2);
    let mut bob2 = Client::new(&mut gw, BOB, "Bob#0012", 4);
    let mut alice = Client::new(&mut gw, ALICE, "Alice#af03", 1);
    let mut carol = Client::new(&mut gw, CAROL, "Carol#0c11", 3);
    alice.say(&mut gw, "/w Bob from alice");
    carol.say(&mut gw, "/w Bob from carol");
    bob1.deliver(&mut gw);
    bob2.deliver(&mut gw);
    alice.deliver(&mut gw);

    let names = |client: &Client| history_files(client);
    let (b1, b2) = (names(&bob1), names(&bob2));
    assert_eq!((b1.len(), b2.len()), (2, 2), "one log per conversation");
    for path in b1.iter().chain(&b2).chain(&names(&alice)) {
        let name = path.strip_prefix("history/").expect("under history/");
        let stem = name.strip_suffix(".log").expect("a .log file");
        assert_eq!(stem.len(), 16, "{name}");
        assert!(
            stem.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')),
            "{name}"
        );
        for word in ["af03", "0012", "0c11", "alice", "bob", "carol", "dm"] {
            assert!(!path.to_lowercase().contains(word), "{path} names {word}");
        }
    }
    assert_ne!(b1[0], b1[1], "each conversation has its own");
    assert!(
        b1.iter().all(|p| !b2.contains(p)),
        "the same conversation is another file on another device"
    );
    // The name is the vault key's derivation of the conversation.
    let key = vault_key(&bob1);
    assert!(b1.contains(&history::file_path(&key, &conv(DM_AB))));
    assert!(b1.contains(&history::file_path(&key, &conv("dm:0012:0c11"))));
    // And the account's two sides differ on the same conversation.
    assert!(!names(&alice).contains(&history::file_path(&key, &conv(DM_AB))));
}

// ---------------------------------------------------------------------------
// The folder on a real file system.
// ---------------------------------------------------------------------------

#[cfg(not(target_arch = "wasm32"))]
mod dir {
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    use crate::social::ChatDisk;
    use crate::social::disk::DirDisk;

    static NEXT: AtomicU32 = AtomicU32::new(0);

    /// A folder under the system's temp directory that is gone when dropped.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new() -> Self {
            let n = NEXT.fetch_add(1, Ordering::Relaxed);
            // The folder does not exist yet: the disk makes it.
            Self(
                std::env::temp_dir()
                    .join(format!("baylee-chat-disk-test-{}-{n}", std::process::id())),
            )
        }

        fn disk(&self) -> DirDisk {
            DirDisk::new(&self.0)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn a_dir_disk_writes_appends_lists_and_removes() {
        let scratch = Scratch::new();
        let mut disk = scratch.disk();
        assert_eq!(disk.read("state.bin"), None, "nothing yet");
        assert!(disk.list("history").is_empty());

        disk.write("state.bin", b"one").unwrap();
        assert_eq!(disk.read("state.bin").unwrap(), b"one");
        disk.write("state.bin", b"two").unwrap();
        assert_eq!(disk.read("state.bin").unwrap(), b"two", "replaced");

        disk.append("history/b.log", b"x").unwrap();
        disk.append("history/b.log", b"yz").unwrap();
        disk.write("history/a.log", b"a").unwrap();
        assert_eq!(disk.read("history/b.log").unwrap(), b"xyz", "appended");
        assert_eq!(disk.list("history"), ["a.log", "b.log"], "sorted");

        // No temp file is left behind by a write.
        let left: Vec<String> = std::fs::read_dir(scratch.0.join("history"))
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        assert_eq!(left.len(), 2, "{left:?}");

        disk.remove("history/a.log").unwrap();
        assert_eq!(disk.read("history/a.log"), None);
        disk.remove("history/a.log").unwrap();
        disk.remove("history/never.log").unwrap();
        assert_eq!(disk.list("history"), ["b.log"]);

        disk.remove_all().unwrap();
        assert!(!scratch.0.exists(), "the whole folder is gone");
        assert_eq!(disk.read("state.bin"), None);
        disk.remove_all().unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn a_dir_disk_makes_its_folders_private_and_its_files_readable_by_the_owner_only() {
        use std::os::unix::fs::PermissionsExt as _;

        let scratch = Scratch::new();
        let mut disk = scratch.disk();
        disk.write("state.bin", b"state").unwrap();
        disk.append("history/h.log", b"log").unwrap();
        disk.write("history/w.log", b"log").unwrap();

        let mode =
            |path: &std::path::Path| std::fs::metadata(path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(&scratch.0), 0o700, "the chat folder");
        assert_eq!(mode(&scratch.0.join("history")), 0o700);
        assert_eq!(mode(&scratch.0.join("state.bin")), 0o600);
        assert_eq!(mode(&scratch.0.join("history/h.log")), 0o600, "appended");
        assert_eq!(mode(&scratch.0.join("history/w.log")), 0o600, "written");
    }

    #[test]
    fn a_dir_disk_refuses_a_path_that_would_leave_its_folder() {
        let scratch = Scratch::new();
        let mut disk = scratch.disk();
        for bad in [
            "", ".", "..", "../x", "a/../b", "/abs", "a//b", "a\\b", "a/",
        ] {
            assert!(disk.write(bad, b"x").is_err(), "{bad:?} written");
            assert!(disk.append(bad, b"x").is_err(), "{bad:?} appended");
            assert!(disk.remove(bad).is_err(), "{bad:?} removed");
            assert_eq!(disk.read(bad), None, "{bad:?} read");
        }
        assert!(!scratch.0.exists(), "a refused path made no folder either");
        let outside = scratch.0.with_file_name("x");
        assert!(!outside.exists());
    }
}
