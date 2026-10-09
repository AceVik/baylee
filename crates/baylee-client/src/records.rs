//! The records of games this client hosted itself, kept on disk (#315).
//!
//! A game against the house runs here (`host::LocalHost`), so no gateway
//! keeps its record. The client keeps the last few itself, gzipped, in
//! `records/` beside its settings, bounded by
//! `bugreport::retention` (count and weight). They stay on this machine:
//! a record leaves it only with a report the player ticked it for, and
//! the form reads the one in memory, never these files. In a browser
//! there is no such folder, and nothing is kept.
//!
//! **Written as the game is played, not when it ends.** The record used to
//! reach the disk at the game's end or when the table was left, from
//! `LocalHost`'s `Drop` — which a release build never runs on a panic
//! (`panic = "abort"`), so the one game a crash report most needed was the
//! one that was lost. Now each step's lines are appended as one gzip member
//! ([`LiveRecord::write`]), the way the engine sends a hosted game's record
//! in pieces: members one after another are one gzip stream, so a file a
//! crash left behind reads as the record up to the step it died in. At the
//! end the file is rewritten as one member ([`LiveRecord::finish`]), which
//! deflates the whole game against itself again. The writer holds an
//! advisory lock on its file while the game is on.
//!
//! [`recover`] runs once at start: a file cut short in the middle of an
//! append (the only thing a crash can leave besides a whole one) is cut back
//! to its whole lines, and one with nothing past its header is removed. A
//! file another running client holds its lock on is left alone.

/// The folder the records live in, beside the settings.
#[cfg(not(target_arch = "wasm32"))]
const FOLDER: &str = "records";

/// The record of the game being played, on disk as it is played.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) struct LiveRecord {
    /// Its file; `None` in a process that has not opened the settings store
    /// (a test), which writes nothing, as everything the store writes.
    path: Option<std::path::PathBuf>,
    /// The file, open for appending and locked, from the first step on.
    file: Option<std::fs::File>,
    /// How many bytes of the record are on disk.
    written: usize,
    /// Whether the file holds the whole record, compacted: nothing more
    /// will be written.
    finished: bool,
}

#[cfg(not(target_arch = "wasm32"))]
impl LiveRecord {
    /// The kept record `name` (`bugreport::record_file_name`), in the
    /// settings store's records folder.
    pub(crate) fn new(name: &str) -> Self {
        Self::in_folder(crate::settings::store::folder(FOLDER), name)
    }

    /// The kept record `name` in `folder`; nothing is written without one.
    pub(crate) fn in_folder(folder: Option<std::path::PathBuf>, name: &str) -> Self {
        let path = folder
            .filter(|_| baylee_client_core::bugreport::is_record_file_name(name))
            .map(|folder| folder.join(name));
        Self {
            path,
            file: None,
            written: 0,
            finished: false,
        }
    }

    /// The kept record `name`, of which the first `written` bytes are on
    /// disk already: a game picked up again after a restart
    /// (`host::LocalHost::resume`) goes on in the file it was kept in.
    pub(crate) fn continuing(name: &str, written: usize) -> Self {
        Self::continuing_in(crate::settings::store::folder(FOLDER), name, written)
    }

    /// [`LiveRecord::continuing`] in `folder`.
    pub(crate) fn continuing_in(
        folder: Option<std::path::PathBuf>,
        name: &str,
        written: usize,
    ) -> Self {
        Self {
            written,
            ..Self::in_folder(folder, name)
        }
    }

    /// Appends what `record` gained since the last write, as one gzip
    /// member. Nothing until it is past its header: a game nobody played a
    /// step of is not worth a file.
    ///
    /// No `fsync` per step: a process that dies keeps what its `write`
    /// handed the system, and only a machine that loses power loses the
    /// last steps, which `finish` syncs.
    pub(crate) fn write(&mut self, record: &[u8]) {
        use std::io::Write as _;
        if self.finished || self.written >= record.len() || !past_header(record) {
            return;
        }
        if self.file.is_none() {
            self.file = self.open();
        }
        let Some(file) = self.file.as_mut() else {
            return;
        };
        let member = baylee_client_core::bugreport::gzip(&record[self.written..]);
        if file.write_all(&member).is_ok() {
            self.written = record.len();
        }
    }

    /// Writes the whole record as one member over the steps appended so far
    /// (atomically: a temporary renamed over it), and keeps nothing more.
    /// Called at the game's end and when the table is left.
    pub(crate) fn finish(&mut self, record: &[u8]) {
        if self.finished || !past_header(record) {
            return;
        }
        self.finished = true;
        // Unlocked and closed before the rename, which Windows refuses over
        // an open file.
        self.file = None;
        let Some(path) = self.path.as_ref() else {
            return;
        };
        crate::settings::store::write_at(path, baylee_client_core::bugreport::gzip(record));
        if let Some(folder) = path.parent() {
            retain(folder);
        }
    }

    /// The file, created for appending (its owner's alone, as every file
    /// the store writes), locked, and the folder trimmed to its bounds.
    fn open(&self) -> Option<std::fs::File> {
        let path = self.path.as_ref()?;
        let folder = path.parent()?;
        let _ = std::fs::create_dir_all(folder);
        let mut options = std::fs::OpenOptions::new();
        // Read as well: Windows locks only through a handle with read or
        // write access, and an append-only one has neither (no
        // `FILE_WRITE_DATA`), so `try_lock` failed there in silence and
        // another client's `recover` cut this game's record back while it
        // was being played (`what_a_crash_left_is_repaired_at_start`).
        options.read(true).append(true).create(true);
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
        let file = options.open(path).ok()?;
        // Advisory, and only for `recover` in another client started while
        // this game is on: a file that cannot be locked is still written.
        let _ = file.try_lock();
        retain(folder);
        Some(file)
    }
}

/// Whether `record` holds a line past its header.
#[cfg(not(target_arch = "wasm32"))]
fn past_header(record: &[u8]) -> bool {
    record
        .iter()
        .position(|b| *b == b'\n')
        .is_some_and(|end| end + 1 < record.len())
}

/// Deletes the oldest records in `folder` past the bounds.
#[cfg(not(target_arch = "wasm32"))]
fn retain(folder: &std::path::Path) {
    for doomed in baylee_client_core::bugreport::retention(&records_in(folder)) {
        let _ = std::fs::remove_file(folder.join(doomed));
    }
}

/// Every file in `folder` with its size.
#[cfg(not(target_arch = "wasm32"))]
fn records_in(folder: &std::path::Path) -> Vec<(String, u64)> {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            let bytes = entry.metadata().ok()?.len();
            Some((name, bytes))
        })
        .collect()
}

/// The kept record `name` as the record it holds, every whole line of it;
/// `None` for a name that is not a record's, or a file that is not there.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn read_kept(name: &str) -> Option<Vec<u8>> {
    read_kept_in(crate::settings::store::folder(FOLDER)?.as_path(), name)
}

/// [`read_kept`] in `folder`.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn read_kept_in(folder: &std::path::Path, name: &str) -> Option<Vec<u8>> {
    if !baylee_client_core::bugreport::is_record_file_name(name) {
        return None;
    }
    let bytes = std::fs::read(folder.join(name)).ok()?;
    baylee_client_core::bugreport::inflate(&bytes)
}

/// Repairs what a crash left of the kept records, once, at start.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn recover() {
    if let Some(folder) = crate::settings::store::folder(FOLDER) {
        recover_in(&folder);
    }
}

/// [`recover`] in `folder`: each record cut short is cut back to its whole
/// lines, and one with nothing past its header removed. A whole file, and
/// one another client holds the lock on, is left as it is.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn recover_in(folder: &std::path::Path) {
    use baylee_client_core::bugreport::{ReadBack, gzip, is_record_file_name, read_back};
    use std::io::Read as _;
    for (name, _) in records_in(folder) {
        if !is_record_file_name(&name) {
            continue;
        }
        let path = folder.join(&name);
        let Ok(mut file) = std::fs::File::open(&path) else {
            continue;
        };
        if let Err(std::fs::TryLockError::WouldBlock) = file.try_lock() {
            continue;
        }
        let mut bytes = Vec::new();
        if file.read_to_end(&mut bytes).is_err() {
            continue;
        }
        drop(file);
        match read_back(&bytes) {
            ReadBack::Whole => {}
            ReadBack::Torn(lines) => crate::settings::store::write_at(&path, gzip(&lines)),
            ReadBack::Empty => {
                let _ = std::fs::remove_file(&path);
            }
        }
    }
}

/// A browser keeps no records.
#[cfg(target_arch = "wasm32")]
pub(crate) struct LiveRecord;

#[cfg(target_arch = "wasm32")]
impl LiveRecord {
    pub(crate) fn new(_name: &str) -> Self {
        Self
    }

    pub(crate) fn continuing(_name: &str, _written: usize) -> Self {
        Self
    }

    pub(crate) fn write(&mut self, _record: &[u8]) {}

    pub(crate) fn finish(&mut self, _record: &[u8]) {}
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use baylee_client_core::bugreport::{ReadBack, gzip, read_back, record_file_name};

    const HEADER: &[u8] = b"{\"kind\":\"header\",\"record\":1}\n";

    fn input(n: u32) -> String {
        format!("{{\"kind\":\"input\",\"n\":{n},\"action\":\"Pass\"}}\n")
    }

    /// A folder of this test's own, gone when it is dropped.
    struct Scratch(std::path::PathBuf);

    impl Scratch {
        fn new(tag: &str) -> Self {
            let dir =
                std::env::temp_dir().join(format!("baylee-records-{tag}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).expect("a scratch folder");
            Self(dir)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn unzip(bytes: &[u8]) -> String {
        use std::io::Read as _;
        let mut text = String::new();
        flate2::read::MultiGzDecoder::new(bytes)
            .read_to_string(&mut text)
            .expect("one gzip stream");
        text
    }

    /// The file is on disk from the first step and holds every step as it
    /// goes, so a client killed mid-game leaves the game so far; the end
    /// rewrites it as one member of the same lines.
    #[test]
    fn a_record_is_on_disk_from_the_first_step_on() {
        let scratch = Scratch::new("live");
        let name = record_file_name(1_759_000_000_000, 7);
        let path = scratch.0.join(&name);
        let mut live = LiveRecord::in_folder(Some(scratch.0.clone()), &name);

        let mut record = HEADER.to_vec();
        live.write(&record);
        assert!(!path.exists(), "a header alone is not worth a file");

        // Read through the writer's own handle while it holds the file:
        // Windows' locks are mandatory, and refuse any other handle a read.
        let on_disk = |live: &mut LiveRecord| {
            use std::io::{Read as _, Seek as _};
            let file = live.file.as_mut().expect("open");
            file.seek(std::io::SeekFrom::Start(0)).expect("seek");
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes).expect("written as it goes");
            bytes
        };
        let mut members = 0;
        for n in 0..5 {
            record.extend(input(n).as_bytes());
            live.write(&record);
            members += 1;
            let on_disk = on_disk(&mut live);
            assert_eq!(read_back(&on_disk), ReadBack::Whole);
            assert_eq!(unzip(&on_disk).as_bytes(), &record[..], "after step {n}");
        }
        let appended = on_disk(&mut live).len();

        live.finish(&record);
        let finished = std::fs::read(&path).expect("the file");
        assert_eq!(unzip(&finished).as_bytes(), &record[..]);
        assert_eq!(finished, gzip(&record), "one member at the end");
        assert!(
            finished.len() < appended,
            "{members} members ({appended} bytes) compact to {}",
            finished.len()
        );
        live.write(&[record.clone(), input(9).into_bytes()].concat());
        assert_eq!(
            std::fs::read(&path).expect("the file"),
            finished,
            "finished"
        );
    }

    /// A crash in the middle of an append is cut back to the whole lines
    /// before it; a file with nothing past its header goes; a whole file and
    /// one another client is writing are left alone.
    #[test]
    fn what_a_crash_left_is_repaired_at_start() {
        let scratch = Scratch::new("recover");
        let mut record = HEADER.to_vec();
        record.extend(input(0).as_bytes());
        let one = gzip(&record);
        let mut torn = one.clone();
        torn.extend(gzip(input(1).as_bytes()));
        torn.truncate(one.len() + 5);

        let at = |i: u64| scratch.0.join(record_file_name(1_000 + i, i));
        std::fs::write(at(0), &torn).expect("written");
        std::fs::write(at(1), &gzip(HEADER)[..10]).expect("written");
        std::fs::write(at(2), &one).expect("written");
        std::fs::write(scratch.0.join("notes.txt"), b"not a record").expect("written");

        // The live one: locked by a writer of its own, and torn on purpose
        // so that a repair would be seen.
        // The tear goes through the writer's own handle, as a crash in the
        // middle of its append would leave it: Windows' locks are mandatory,
        // so no other handle may write or read the file while it is held,
        // which is also why it is read back only once the writer is gone.
        let live_name = record_file_name(2_000, 9);
        let mut live = LiveRecord::in_folder(Some(scratch.0.clone()), &live_name);
        live.write(&record);
        let live_path = scratch.0.join(&live_name);
        let tail = gzip(input(1).as_bytes())[..4].to_vec();
        {
            use std::io::Write as _;
            live.file
                .as_mut()
                .expect("open")
                .write_all(&tail)
                .expect("append");
        }
        let mut live_before = gzip(&record);
        live_before.extend(&tail);

        recover_in(&scratch.0);

        assert_eq!(
            unzip(&std::fs::read(at(0)).expect("repaired")).as_bytes(),
            &record[..],
            "cut back to its whole lines"
        );
        assert!(!at(1).exists(), "nothing past the header: removed");
        assert_eq!(std::fs::read(at(2)).expect("whole"), one, "left alone");
        assert!(scratch.0.join("notes.txt").exists(), "not a record");
        drop(live);
        assert_eq!(
            std::fs::read(&live_path).expect("live"),
            live_before,
            "a game another client is playing is not touched"
        );
    }
}
