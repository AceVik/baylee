//! Where the chat is kept on disk: the folder's name, the log's frames, and
//! the two disks, which must behave alike (§6).

use crate::social::Effect;
use crate::social::disk::{
    ChatDisk, DirDisk, HISTORY_DIR, MemoryDisk, STATE_FILE, apply, chat_folder, frame, frames,
    history_path,
};

/// A folder of its own, removed when the test is over.
struct Folder(std::path::PathBuf);

impl Folder {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!("baylee-chatdisk-{}", uuid::Uuid::now_v7())))
    }

    fn disk(&self) -> DirDisk {
        DirDisk::new(self.0.join("chat"))
    }

    /// Every file under the folder, hidden ones included.
    fn files(&self) -> Vec<String> {
        fn walk(dir: &std::path::Path, out: &mut Vec<String>) {
            for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
                if entry.path().is_dir() {
                    walk(&entry.path(), out);
                } else {
                    out.push(entry.file_name().into_string().unwrap());
                }
            }
        }
        let mut out = Vec::new();
        walk(&self.0, &mut out);
        out.sort();
        out
    }
}

impl Drop for Folder {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Runs `check` on both disks.
fn on_both(check: impl Fn(&mut dyn ChatDisk)) {
    check(&mut MemoryDisk::new());
    let folder = Folder::new();
    check(&mut folder.disk());
}

#[test]
fn a_file_reads_back_as_written_and_the_newer_write_wins() {
    on_both(|disk| {
        assert_eq!(disk.read(STATE_FILE), None);
        disk.write(STATE_FILE, b"one, and longer").unwrap();
        disk.write(STATE_FILE, b"two").unwrap();
        assert_eq!(disk.read(STATE_FILE).unwrap(), b"two");
    });
}

#[test]
fn appending_makes_the_log_and_then_adds_to_its_end() {
    on_both(|disk| {
        let log = history_path("a.log");
        disk.append(&log, b"one").unwrap();
        disk.append(&log, b"two").unwrap();
        assert_eq!(disk.read(&log).unwrap(), b"onetwo");
    });
}

#[test]
fn a_folder_lists_its_own_files_by_name_and_no_deeper_ones() {
    on_both(|disk| {
        disk.write(&history_path("b.log"), b"b").unwrap();
        disk.write(&history_path("a.log"), b"a").unwrap();
        disk.write(STATE_FILE, b"s").unwrap();
        assert_eq!(disk.list(HISTORY_DIR), ["a.log", "b.log"]);
        assert_eq!(disk.list(""), [STATE_FILE]);
        assert!(disk.list("nowhere").is_empty());
    });
}

#[test]
fn removing_takes_one_file_and_is_no_error_twice() {
    on_both(|disk| {
        disk.write(&history_path("a.log"), b"a").unwrap();
        disk.write(&history_path("b.log"), b"b").unwrap();
        disk.remove(&history_path("a.log")).unwrap();
        disk.remove(&history_path("a.log")).unwrap();
        assert_eq!(disk.list(HISTORY_DIR), ["b.log"]);
    });
}

#[test]
fn removing_everything_leaves_nothing_and_is_no_error_twice() {
    on_both(|disk| {
        disk.write(&history_path("a.log"), b"a").unwrap();
        disk.write(STATE_FILE, b"s").unwrap();
        disk.remove_all().unwrap();
        disk.remove_all().unwrap();
        assert_eq!(disk.read(STATE_FILE), None);
        assert!(disk.list(HISTORY_DIR).is_empty());
    });
}

#[test]
fn no_path_leaves_the_chat_folder() {
    let folder = Folder::new();
    let mut disk = folder.disk();
    disk.write(STATE_FILE, b"inside").unwrap();
    std::fs::write(folder.0.join("outside"), b"secret").unwrap();
    for path in [
        "",
        "..",
        "../outside",
        "./outside",
        "history/../../outside",
        "/etc/passwd",
        "history//a.log",
        "history/",
        "..\\outside",
        "history\\a.log",
    ] {
        assert_eq!(disk.read(path), None, "{path}");
        assert!(disk.write(path, b"x").is_err(), "{path}");
        assert!(disk.append(path, b"x").is_err(), "{path}");
        assert!(disk.remove(path).is_err(), "{path}");
        if !path.is_empty() {
            assert!(disk.list(path).is_empty(), "{path}");
        }
    }
    assert_eq!(folder.files(), ["outside", STATE_FILE]);
    assert_eq!(std::fs::read(folder.0.join("outside")).unwrap(), b"secret");
}

#[test]
fn removing_everything_stays_inside_the_chat_folder() {
    let folder = Folder::new();
    let mut disk = folder.disk();
    disk.write(STATE_FILE, b"inside").unwrap();
    std::fs::write(folder.0.join("outside"), b"kept").unwrap();
    disk.remove_all().unwrap();
    assert_eq!(folder.files(), ["outside"]);
}

#[test]
fn a_write_leaves_no_temporary_file_behind() {
    let folder = Folder::new();
    let mut disk = folder.disk();
    disk.write(STATE_FILE, b"s").unwrap();
    assert_eq!(folder.files(), [STATE_FILE]);
    // A folder stands where the file should go, so the rename fails.
    std::fs::create_dir_all(folder.0.join("chat/history/a.log")).unwrap();
    assert!(disk.write(&history_path("a.log"), b"a").is_err());
    assert_eq!(folder.files(), [STATE_FILE], "nor does a failed one");
}

#[test]
fn a_half_written_temporary_file_is_not_listed() {
    let folder = Folder::new();
    let mut disk = folder.disk();
    disk.write(STATE_FILE, b"s").unwrap();
    std::fs::write(folder.0.join("chat/.state.bin.1.tmp"), b"half").unwrap();
    assert_eq!(disk.list(""), [STATE_FILE]);
}

#[cfg(unix)]
#[test]
fn only_the_player_may_read_what_the_chat_keeps() {
    use std::os::unix::fs::PermissionsExt as _;
    let folder = Folder::new();
    let mut disk = folder.disk();
    disk.write(STATE_FILE, b"s").unwrap();
    disk.append(&history_path("a.log"), b"a").unwrap();
    let mode = |path: &str| {
        let meta = std::fs::metadata(folder.0.join("chat").join(path)).unwrap();
        meta.permissions().mode() & 0o777
    };
    assert_eq!(mode(STATE_FILE), 0o600);
    assert_eq!(mode("history/a.log"), 0o600);
    assert_eq!(mode("history"), 0o700);
    assert_eq!(mode(""), 0o700);
}

#[test]
fn the_frames_of_a_log_come_back_one_by_one() {
    let records: [&[u8]; 3] = [b"first", b"", b"third record"];
    let log: Vec<u8> = records.iter().flat_map(|r| frame(r)).collect();
    assert_eq!(frames(&log), records);
    assert!(frames(b"").is_empty());
}

#[test]
fn a_log_torn_at_any_byte_keeps_every_whole_record_before_the_tear() {
    let first = frame(b"first");
    let log = [first.clone(), frame(b"second")].concat();
    for cut in 0..log.len() {
        let whole = usize::from(cut >= first.len());
        assert_eq!(frames(&log[..cut]).len(), whole, "torn at {cut}");
    }
    assert_eq!(frames(&log).len(), 2);
}

#[test]
fn a_frame_that_claims_four_gigabytes_is_the_end_of_the_log() {
    let mut log = frame(b"kept");
    log.extend_from_slice(&u32::MAX.to_be_bytes());
    log.extend_from_slice(b"rest");
    assert_eq!(frames(&log), [b"kept"]);
}

#[test]
fn each_disk_effect_does_its_own_work_and_no_other_effect_touches_the_disk() {
    let mut disk = MemoryDisk::new();
    let file = history_path("a.log");
    let mut run = |effect: Effect| apply(&effect, &mut disk).map(Result::unwrap);
    assert!(run(Effect::SaveState(b"state".to_vec())).is_some());
    for record in [b"one".to_vec(), b"two".to_vec()] {
        let file = file.clone();
        assert!(run(Effect::AppendHistory { file, record }).is_some());
    }
    assert_eq!(disk.read(STATE_FILE).unwrap(), b"state");
    assert_eq!(disk.read(&file).unwrap(), b"onetwo");
    let rewrite = |bytes: &[u8]| Effect::RewriteHistory {
        file: file.clone(),
        bytes: bytes.to_vec(),
    };
    apply(&rewrite(b"two"), &mut disk).unwrap().unwrap();
    assert_eq!(disk.read(&file).unwrap(), b"two");
    apply(&rewrite(b""), &mut disk).unwrap().unwrap();
    assert_eq!(disk.read(&file), None, "an empty log is no log");
    apply(&Effect::DeleteAllLocal, &mut disk).unwrap().unwrap();
    assert!(disk.files.is_empty());
}

#[test]
fn one_gateway_has_one_chat_folder_however_its_address_is_typed() {
    let folder = chat_folder("https://play.example", "AB12");
    for spelt in [
        "https://play.example/",
        "https://PLAY.example",
        "  https://play.example// ",
    ] {
        assert_eq!(chat_folder(spelt, "AB12"), folder, "{spelt}");
    }
    assert_ne!(chat_folder("https://play.example:8443", "AB12"), folder);
    assert_ne!(chat_folder("http://play.example", "AB12"), folder);
    assert_ne!(chat_folder("https://play.example", "AB13"), folder);
    let parts: Vec<&str> = folder.split('/').collect();
    assert_eq!(parts[0], "chat");
    assert_eq!(parts[1].len(), 12);
    assert!(parts[1].bytes().all(|b| b.is_ascii_hexdigit()));
    assert_eq!(parts[2], "AB12");
}

#[test]
fn a_tag_from_a_hostile_gateway_cannot_climb_out_of_the_chat_folder() {
    let folder = chat_folder("https://play.example", "../../.ssh/AB\\12\0");
    let tag = folder.rsplit('/').next().unwrap();
    assert_eq!(tag, "sshAB12");
    assert_eq!(folder.matches('/').count(), 2);
}
