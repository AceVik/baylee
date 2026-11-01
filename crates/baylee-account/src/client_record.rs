//! A record a client wrote of a game it hosted itself, as whoever takes one
//! from it checks it before keeping it: the feedback service on
//! `POST /client/reports` and a `client` record passed on to it, and the
//! official gateway before it files one with Central, which never unpacks a
//! part (baylee-central `docs/reports.md` §"Privacy invariants").

use std::io::Read as _;

/// The largest record a client may attach, compressed (either route):
/// Central's own bound on a client's record.
pub const MAX_CLIENT_RECORD_BYTES: usize = 4 * 1024 * 1024;

const _: () = assert!(
    MAX_CLIENT_RECORD_BYTES == crate::reports::MAX_CLIENT_RECORD_GZIP as usize,
    "one bound for a client's record, here and at Central"
);

/// The most a client's record may unpack to: a bound on what reading it
/// costs, so a small stream that unpacks to gigabytes is refused, not read.
pub const MAX_CLIENT_RECORD_UNPACKED: u64 = 32 * 1024 * 1024;

/// Why a client's record is refused.
#[derive(Debug, PartialEq, Eq)]
pub enum NotARecord {
    /// Over [`MAX_CLIENT_RECORD_BYTES`] compressed.
    TooLarge,
    /// Over [`MAX_CLIENT_RECORD_UNPACKED`] unpacked.
    UnpacksTooLarge,
    /// Not gzip, or not text.
    Unreadable,
    /// It does not read as a game record (`docs/protocol.md` §"The game
    /// record"): a header first and only there, every line one of the
    /// record's lines.
    NotARecord,
}

/// Checks that `gzip` reads as a game record, without the engine: a
/// header of the record version this service knows first, and only once,
/// then inputs, chair changes and an end, each a JSON object. Whether it
/// replays is for the build its header names (`baylee_gamehost::record`),
/// and nothing here claims it does.
///
/// # Errors
///
/// [`NotARecord`] says which way it is not one.
pub fn check_record(gzip: &[u8]) -> Result<(), NotARecord> {
    if gzip.len() > MAX_CLIENT_RECORD_BYTES {
        return Err(NotARecord::TooLarge);
    }
    let mut text = String::new();
    flate2::read::GzDecoder::new(gzip)
        .take(MAX_CLIENT_RECORD_UNPACKED + 1)
        .read_to_string(&mut text)
        .map_err(|_| NotARecord::Unreadable)?;
    if text.len() as u64 > MAX_CLIENT_RECORD_UNPACKED {
        return Err(NotARecord::UnpacksTooLarge);
    }
    let mut lines = text.lines().filter(|l| !l.trim().is_empty());
    let header: serde_json::Value = lines
        .next()
        .and_then(|l| serde_json::from_str(l).ok())
        .ok_or(NotARecord::NotARecord)?;
    let is_header = header.get("kind").and_then(serde_json::Value::as_str) == Some("header")
        && header.get("record").and_then(serde_json::Value::as_u64) == Some(1)
        && header
            .get("build")
            .is_some_and(serde_json::Value::is_string);
    if !is_header {
        return Err(NotARecord::NotARecord);
    }
    for line in lines {
        let value: serde_json::Value =
            serde_json::from_str(line).map_err(|_| NotARecord::NotARecord)?;
        let kind = value.get("kind").and_then(serde_json::Value::as_str);
        if !matches!(kind, Some("input" | "chair" | "end")) {
            return Err(NotARecord::NotARecord);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    fn gz(text: &str) -> Vec<u8> {
        let mut out = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        out.write_all(text.as_bytes()).unwrap();
        out.finish().unwrap()
    }

    const HEADER: &str = r#"{"kind":"header","record":1,"build":"0.1.0","preset":{},"hash":"00"}"#;

    /// A record's shape is read without the engine: a header first, then
    /// only the record's own lines.
    #[test]
    fn a_record_is_its_header_and_its_own_lines() {
        let input =
            r#"{"kind":"input","n":0,"at":0,"seat":0,"by":"seat","action":"Pass","hash":"01"}"#;
        assert_eq!(check_record(&gz(&format!("{HEADER}\n{input}\n"))), Ok(()));
        assert_eq!(check_record(&gz(HEADER)), Ok(()));
        for bad in [
            String::new(),
            input.to_owned(),
            format!("{HEADER}\n{HEADER}\n"),
            format!("{HEADER}\nnot json\n"),
            r#"{"kind":"header","record":2,"build":"x"}"#.to_owned(),
        ] {
            assert_eq!(
                check_record(&gz(&bad)),
                Err(NotARecord::NotARecord),
                "{bad}"
            );
        }
        assert_eq!(check_record(b"plain"), Err(NotARecord::Unreadable));
    }

    /// A record that carries `seen` on its header and inputs and `turn` on
    /// its end (`baylee_gamehost::record::Line`) is a record: the shape
    /// check reads kinds, never the keys beside them, and the keys stay
    /// there whatever else is on the line.
    #[test]
    fn a_record_with_seen_and_an_end_turn_is_still_a_record() {
        let header = r#"{"kind":"header","record":1,"build":"0.1.0","preset":{},"hash":"00","seen":"30b17ab8dd2e7087"}"#;
        let input = r#"{"kind":"input","n":0,"at":0,"seat":0,"by":"seat","action":"PassPriority","hash":"01","seen":"deb49d0b74b15d90"}"#;
        let end = r#"{"kind":"end","n":1,"at":0,"winners":[1],"reason":"concede","turn":7}"#;
        let record = format!("{header}\n{input}\n{end}\n");
        assert_eq!(check_record(&gz(&record)), Ok(()));
        // The same lines, the keys beside them gone: the same answer.
        let older = record
            .replace(r#","seen":"30b17ab8dd2e7087""#, "")
            .replace(r#","seen":"deb49d0b74b15d90""#, "")
            .replace(r#","turn":7"#, "");
        assert!(!older.contains("seen") && !older.contains("turn"));
        assert_eq!(check_record(&gz(&older)), Ok(()));
        // The check is still a check: a header's `seen` does not make a
        // header of a line of another kind.
        let imposter = format!("{header}\n{header}\n");
        assert_eq!(check_record(&gz(&imposter)), Err(NotARecord::NotARecord));
    }

    /// A small stream that unpacks past the bound is refused before it is
    /// read whole.
    #[test]
    fn a_record_that_unpacks_past_its_bound_is_refused() {
        let line = format!("{HEADER}\n");
        let mut body = line.clone();
        let filler = " ".repeat(1024 * 1024);
        while (body.len() as u64) <= MAX_CLIENT_RECORD_UNPACKED {
            body.push_str(&filler);
        }
        let packed = gz(&body);
        assert!(packed.len() < MAX_CLIENT_RECORD_BYTES);
        assert_eq!(check_record(&packed), Err(NotARecord::UnpacksTooLarge));
    }
}
