//! The record of a game this device hosted itself, and how many it keeps.
//!
//! A game against the house runs in the client (`LocalHost`), so no gateway
//! keeps its record (#315) and a report about it would arrive without the
//! one thing that replays it. The client writes the same JSON Lines record
//! a hosted game's engine writes (`baylee_gamehost::record`), keeps the
//! last few on disk ([`retention`]), and a report may carry the current
//! one, gzipped, when the player ticks the box for that report.
//!
//! What a record is matters for the form, which says so: every input from
//! the seed on, which is every seat's cards, the hidden ones included. It
//! names nobody and holds no token; [`LocalRecord::plain`] is kept beside
//! the gzip so that the report's leak check ([`super::seal`]) reads the
//! record too rather than a base64 of a gzip it could see nothing in.

use std::io::Write as _;
use std::sync::Arc;

use serde::ser::SerializeStruct as _;

/// The most a record may weigh, gzipped, to ride with a report: the bound
/// the gateway and the feedback service both hold a client's record to.
pub const MAX_RECORD_BYTES: usize = 4 * 1024 * 1024;

/// How many records this device keeps.
pub const KEEP_RECORDS: usize = 20;

/// How much they may weigh together, gzipped, on disk.
pub const KEEP_RECORD_BYTES: u64 = 64 * 1024 * 1024;

/// What a kept record's file is called: `game-<ms>-<seed>.jsonl.gz`, the
/// moment it started in milliseconds since the Unix epoch, padded so that
/// names sort as the games started, and the preset's seed telling apart
/// two that started in one millisecond.
#[must_use]
pub fn record_file_name(started_ms: u64, seed: u64) -> String {
    format!("game-{started_ms:013}-{seed:016x}.jsonl.gz")
}

/// Whether `name` is one [`record_file_name`] makes. [`retention`] never
/// touches anything else, whatever else is in the folder.
#[must_use]
pub fn is_record_file_name(name: &str) -> bool {
    let Some(core) = name
        .strip_prefix("game-")
        .and_then(|rest| rest.strip_suffix(".jsonl.gz"))
    else {
        return false;
    };
    let Some((ms, seed)) = core.split_once('-') else {
        return false;
    };
    ms.len() == 13
        && seed.len() == 16
        && ms.bytes().all(|b| b.is_ascii_digit())
        && seed.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// Which of the kept records to delete: everything past the newest
/// [`KEEP_RECORDS`], and past [`KEEP_RECORD_BYTES`] together, counting
/// from the newest. The newest is kept whatever it weighs: it is the game
/// being played, or the one a report is about to be written on. `files`
/// is every file in the folder with its size; a name that is not a
/// record's is never named.
#[must_use]
pub fn retention(files: &[(String, u64)]) -> Vec<String> {
    let mut records: Vec<&(String, u64)> = files
        .iter()
        .filter(|(name, _)| is_record_file_name(name))
        .collect();
    records.sort_by(|a, b| b.0.cmp(&a.0));
    let mut total = 0u64;
    let mut doomed = Vec::new();
    for (kept, (name, bytes)) in records.into_iter().enumerate() {
        total = total.saturating_add(*bytes);
        if kept > 0 && (kept >= KEEP_RECORDS || total > KEEP_RECORD_BYTES) {
            doomed.push(name.clone());
        }
    }
    doomed
}

/// A local game's record, packed once for every report that may carry it.
///
/// Gzipped and base64'd when it is gathered, not when a report is
/// previewed or sent: the form rebuilds its submission on each change, and
/// a whole game's record packed on each of those would be a stall per
/// keystroke. Cloning it clones three pointers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalRecord {
    /// Whether the game had ended when it was gathered.
    pub complete: bool,
    /// The JSON Lines, as written, for the leak check.
    plain: Arc<str>,
    /// The gzip, in base64, as the wire carries it.
    gzip_base64: Arc<str>,
    /// The gzip's own size.
    gzip_bytes: usize,
}

impl LocalRecord {
    /// `jsonl`, packed; `None` for an empty record or one that is not text.
    #[must_use]
    pub fn pack(jsonl: &[u8]) -> Option<Self> {
        let plain = std::str::from_utf8(jsonl).ok()?;
        if plain.trim().is_empty() {
            return None;
        }
        let gzip = gzip(jsonl);
        let complete = plain
            .lines()
            .rev()
            .find(|line| !line.trim().is_empty())
            .and_then(|line| serde_json::from_str::<serde_json::Value>(line).ok())
            .is_some_and(|line| line.get("kind").and_then(|k| k.as_str()) == Some("end"));
        Some(Self {
            complete,
            plain: Arc::from(plain),
            gzip_base64: Arc::from(super::base64::encode(&gzip)),
            gzip_bytes: gzip.len(),
        })
    }

    /// The JSON Lines it packs.
    #[must_use]
    pub fn plain(&self) -> &str {
        &self.plain
    }

    /// The gzip's size in bytes.
    #[must_use]
    pub fn gzip_bytes(&self) -> usize {
        self.gzip_bytes
    }

    /// Its size on the wire, in kilobytes, for the form to say.
    #[must_use]
    pub fn kilobytes(&self) -> usize {
        self.gzip_base64.len().div_ceil(1000)
    }

    /// Whether it is small enough to ride with a report at all.
    #[must_use]
    pub fn fits(&self) -> bool {
        self.gzip_bytes <= MAX_RECORD_BYTES
    }
}

/// On the wire: `{"complete": …, "gzip_base64": …}`, the shape the gateway
/// (`local_record`) and the service (`record`) both read.
impl serde::Serialize for LocalRecord {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut out = serializer.serialize_struct("LocalRecord", 2)?;
        out.serialize_field("complete", &self.complete)?;
        out.serialize_field("gzip_base64", &*self.gzip_base64)?;
        out.end()
    }
}

/// `bytes`, gzipped: what the client keeps on disk, and the report sends.
#[must_use]
pub fn gzip(bytes: &[u8]) -> Vec<u8> {
    let mut out = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    // Writing to a `Vec` cannot fail.
    let _ = out.write_all(bytes);
    out.finish().unwrap_or_default()
}

/// What a kept record's file says once read back.
///
/// The client writes a game's record as it is played, one gzip member per
/// step, appended (`records::LiveRecord`), so a client that crashes or is
/// killed leaves every step but the one it died in. One such file read back
/// as a single gzip stream (`gunzip`, [`flate2::read::MultiGzDecoder`]) is
/// the record so far. Only a death in the middle of an append leaves a
/// member cut short, and that tail is what [`read_back`] cuts.
#[derive(Debug, PartialEq, Eq)]
pub enum ReadBack {
    /// Every member is whole: nothing to repair.
    Whole,
    /// The last member was cut short: the whole lines before the cut.
    Torn(Vec<u8>),
    /// Nothing past the header survived, or nothing at all: not a record
    /// worth keeping.
    Empty,
}

/// Reads a kept record's file back ([`ReadBack`]).
#[must_use]
pub fn read_back(gzip: &[u8]) -> ReadBack {
    use std::io::Read as _;
    let mut lines = Vec::new();
    let whole = flate2::read::MultiGzDecoder::new(gzip)
        .read_to_end(&mut lines)
        .is_ok();
    // A cut member may already have inflated part of a line: only the
    // lines that end are the record.
    let kept = lines
        .iter()
        .rposition(|b| *b == b'\n')
        .map_or(0, |end| end + 1);
    lines.truncate(kept);
    let past_header = lines
        .iter()
        .position(|b| *b == b'\n')
        .is_some_and(|end| end + 1 < lines.len());
    match (whole, past_header) {
        (true, _) if !gzip.is_empty() => ReadBack::Whole,
        (_, true) => ReadBack::Torn(lines),
        _ => ReadBack::Empty,
    }
}

#[cfg(test)]
mod tests {
    use std::io::Read as _;

    use super::*;

    /// A record written a member per step reads back whole as one stream;
    /// cut anywhere inside its last member it reads back as the whole lines
    /// before the cut, header included; cut inside the header it is no
    /// record at all.
    #[test]
    fn a_record_cut_short_reads_back_as_its_whole_lines() {
        let input =
            r#"{"kind":"input","n":1,"at":0,"seat":1,"by":"seat","action":"Pass","hash":"02"}"#;
        let mut file = gzip(RECORD.as_bytes());
        let first = file.len();
        file.extend(gzip(format!("{input}\n").as_bytes()));
        assert_eq!(read_back(&file), ReadBack::Whole);
        let mut stream = String::new();
        flate2::read::MultiGzDecoder::new(&file[..])
            .read_to_string(&mut stream)
            .expect("one stream");
        assert_eq!(stream, format!("{RECORD}{input}\n"));

        // Cut in its deflate data, the last member gives no whole line; cut
        // in its eight-byte trailer, its line is already whole, and kept.
        let both = format!("{RECORD}{input}\n");
        for cut in first + 1..file.len() {
            let ReadBack::Torn(lines) = read_back(&file[..cut]) else {
                panic!("cut at {cut} of {}: not torn", file.len())
            };
            assert!(
                lines == RECORD.as_bytes() || lines == both.as_bytes(),
                "cut at {cut} of {}: {}",
                file.len(),
                String::from_utf8_lossy(&lines)
            );
        }
        assert_eq!(
            read_back(&file[..=first]),
            ReadBack::Torn(RECORD.as_bytes().to_vec()),
            "cut just past the first member"
        );
        for cut in 0..first {
            let header_only = gzip(
                RECORD
                    .lines()
                    .next()
                    .map(|h| format!("{h}\n"))
                    .unwrap_or_default()
                    .as_bytes(),
            );
            assert_eq!(read_back(&header_only), ReadBack::Whole);
            assert!(
                matches!(read_back(&file[..cut]), ReadBack::Empty | ReadBack::Torn(_)),
                "cut at {cut}"
            );
            if let ReadBack::Torn(lines) = read_back(&file[..cut]) {
                assert!(RECORD.as_bytes().starts_with(&lines), "cut at {cut}");
            }
        }
        assert_eq!(read_back(&[]), ReadBack::Empty);
        assert_eq!(read_back(b"not a gzip"), ReadBack::Empty);
    }

    const RECORD: &str = concat!(
        r#"{"kind":"header","record":1,"build":"0.1.0","preset":{},"hash":"00"}"#,
        "\n",
        r#"{"kind":"input","n":0,"at":0,"seat":0,"by":"seat","action":"Pass","hash":"01"}"#,
        "\n",
    );

    fn unzip(gzip: &[u8]) -> String {
        let mut text = String::new();
        flate2::read::GzDecoder::new(gzip)
            .read_to_string(&mut text)
            .expect("gzip");
        text
    }

    /// Packed once, it is the record as written, gzipped, and says whether
    /// the game was over.
    #[test]
    fn a_record_packs_to_the_lines_it_was_written_as() {
        let open = LocalRecord::pack(RECORD.as_bytes()).expect("a record");
        assert!(!open.complete);
        assert_eq!(open.plain(), RECORD);
        let wire = serde_json::to_value(&open).expect("json");
        assert_eq!(wire["complete"], false);
        let gzip = crate::bugreport::base64::decode_for_tests(
            wire["gzip_base64"].as_str().expect("base64"),
        );
        assert_eq!(gzip.len(), open.gzip_bytes());
        assert_eq!(unzip(&gzip), RECORD);
        assert_eq!(wire.as_object().map(serde_json::Map::len), Some(2));

        let ended = format!(
            "{RECORD}{}\n",
            r#"{"kind":"end","n":1,"at":0,"winners":[0]}"#
        );
        assert!(
            LocalRecord::pack(ended.as_bytes())
                .expect("a record")
                .complete
        );
        assert_eq!(LocalRecord::pack(b"\n  \n"), None);
        assert_eq!(LocalRecord::pack(&[0xff, 0xfe]), None);
    }

    /// The bound is the services' own, from both sides.
    #[test]
    fn a_record_fits_up_to_the_bound_the_services_hold_it_to() {
        let mut record = LocalRecord::pack(RECORD.as_bytes()).expect("a record");
        record.gzip_bytes = MAX_RECORD_BYTES;
        assert!(record.fits());
        record.gzip_bytes += 1;
        assert!(!record.fits());
    }

    #[test]
    fn a_record_file_name_sorts_as_the_games_started() {
        let early = record_file_name(999, 7);
        let late = record_file_name(1_759_000_000_000, 0);
        assert_eq!(early, "game-0000000000999-0000000000000007.jsonl.gz");
        assert!(early < late);
        for name in [&early, &late] {
            assert!(is_record_file_name(name), "{name}");
        }
        for name in [
            "client-settings.json",
            "game-0000000000999-0000000000000007.jsonl",
            "game-999-0000000000000007.jsonl.gz",
            "game-0000000000999-000000000000000G.jsonl.gz",
            "game-0000000000999.jsonl.gz",
        ] {
            assert!(!is_record_file_name(name), "{name}");
        }
    }

    /// The newest [`KEEP_RECORDS`] stay, then the weight decides, newest
    /// first; the newest stays whatever it weighs, and nothing that is not
    /// a record is ever named.
    #[test]
    fn retention_keeps_the_newest_by_count_and_by_weight() {
        let named = |i: u64, bytes: u64| (record_file_name(1_000 + i, i), bytes);
        let many: Vec<(String, u64)> = (0..25).map(|i| named(i, 10)).collect();
        let mut doomed = retention(&many);
        doomed.sort();
        let want: Vec<String> = (0..5).map(|i| named(i, 0).0).collect();
        assert_eq!(doomed, want, "the five oldest go");
        assert!(
            retention(&many[..KEEP_RECORDS]).is_empty(),
            "at the count, none"
        );

        let half = KEEP_RECORD_BYTES / 2;
        let heavy = vec![named(1, half), named(2, half), named(3, 1)];
        assert_eq!(retention(&heavy), vec![named(1, 0).0], "past the weight");
        let at = vec![named(1, half - 1), named(2, half), named(3, 1)];
        assert!(retention(&at).is_empty(), "at the weight, none");

        let huge = vec![named(1, 1), named(2, KEEP_RECORD_BYTES * 2)];
        assert_eq!(retention(&huge), vec![named(1, 0).0], "the newest stays");

        let mut mixed = many.clone();
        mixed.push(("client-settings.json".into(), 1));
        mixed.push(("crash-report.json".into(), 1));
        assert!(
            retention(&mixed)
                .iter()
                .all(|name| is_record_file_name(name)),
            "only records"
        );
    }
}
