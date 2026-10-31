//! A report filed with Baylee Central instead of the feedback service, when
//! the operator says so (`BAYLEE_REPORTS_TO=central`; baylee-central
//! `docs/reports.md` §"Intake"): the metadata first, under the gateway's own
//! Central credential, then what is heavy as the gzip Central keeps and
//! never unpacks: the `client` object, and the record of the game the report
//! is about.
//!
//! What goes into the metadata is brought within Central's bounds here, so a
//! long crash message or a stray control character in what a client wrote
//! never costs the player the whole report: the text keeps its lines and
//! loses only what Central would refuse (`baylee_account::reports::shown`),
//! the build and a crash's message and place come from the client's own
//! `client.build` and `client.crash` and are cut to fit, and a part larger
//! than Central takes goes without, with a line in the log.

use std::io::Write as _;

use baylee_account::reports::{
    self as intake, Announced, AnnouncedRecord, Channel, Crash, GatewaySays, Part, RecordOrigin,
    ReportIntake,
};

use super::ReportKind;

/// How many reports may be on their way to Central at once: each holds its
/// parts in memory (a record may be 64 MiB), and Central itself takes two
/// uploads at a time.
pub(super) const IN_FLIGHT: usize = 4;

/// A game's record as a report carries it, gzip.
pub(super) struct RecordBytes {
    /// Whether the game's end is in it.
    pub(super) complete: bool,
    /// Who wrote it.
    pub(super) origin: RecordOrigin,
    /// The record, gzip.
    pub(super) gzip: Vec<u8>,
}

/// A report as Central is sent it: the metadata, and the parts it announces.
pub(super) struct Filing {
    pub(super) intake: ReportIntake,
    client: Option<Vec<u8>>,
    record: Option<Vec<u8>>,
}

/// What the player sent, within Central's bounds; or why it cannot be.
pub(super) struct Said<'a> {
    pub(super) gateway: GatewaySays,
    /// How it came: a signed-in player's, or straight from a client.
    pub(super) channel: Channel,
    pub(super) reporter: String,
    pub(super) kind: ReportKind,
    pub(super) text: &'a str,
    pub(super) game_id: Option<&'a str>,
    pub(super) client: &'a serde_json::Value,
    /// The build the body names (a direct report's); else the client's own.
    pub(super) build: Option<intake::Build>,
}

const fn kind(kind: ReportKind) -> intake::ReportKind {
    match kind {
        ReportKind::Bug => intake::ReportKind::Bug,
        ReportKind::Improvement => intake::ReportKind::Improvement,
        ReportKind::Feedback => intake::ReportKind::Feedback,
        ReportKind::Crash => intake::ReportKind::Crash,
        ReportKind::Other => intake::ReportKind::Other,
    }
}

/// A crash's message and place, only for a crash report.
fn crash(kind: ReportKind, client: &serde_json::Value) -> Option<Crash> {
    (kind == ReportKind::Crash)
        .then(|| intake::crash_of(client))
        .flatten()
}

/// `client` as the gzip of its JSON, when it says anything and fits.
fn client_part(client: &serde_json::Value) -> Option<Vec<u8>> {
    if client.as_object().is_some_and(serde_json::Map::is_empty) {
        return None;
    }
    let json = serde_json::to_vec(client).ok()?;
    let mut gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    gzip.write_all(&json).ok()?;
    let gzip = gzip.finish().ok()?;
    if gzip.len() > intake::MAX_CLIENT_GZIP as usize {
        tracing::warn!(
            bytes = gzip.len(),
            "a report's client details are larger than Central takes; filed without them"
        );
        return None;
    }
    Some(gzip)
}

/// A record Central takes: gzip, and no larger than its origin's bound.
fn record_part(record: RecordBytes) -> Option<RecordBytes> {
    let max = match record.origin {
        RecordOrigin::Gateway => intake::MAX_RECORD_GZIP,
        RecordOrigin::Client => intake::MAX_CLIENT_RECORD_GZIP,
    };
    if !record.gzip.starts_with(&[0x1f, 0x8b]) || record.gzip.len() > max as usize {
        tracing::warn!(
            bytes = record.gzip.len(),
            "a report's record is not one Central takes; filed without it"
        );
        return None;
    }
    Some(record)
}

/// The report Central is sent, its parts compressed and announced; or why
/// the player's report cannot be filed (`400`).
pub(super) fn prepare(
    said: &Said<'_>,
    record: Option<RecordBytes>,
) -> Result<Filing, &'static str> {
    let crash = crash(said.kind, said.client);
    // A crash report's text is the first line of its panic (the client's
    // `crash_submission`), which can be empty: Central takes none, so it
    // reads the crash's own message instead (`text_of`).
    let text = intake::text_of(said.text, kind(said.kind), crash.as_ref())
        .ok_or("the report has no text")?;
    if said
        .game_id
        .is_some_and(|g| !g.chars().all(|c| c.is_ascii_graphic()))
    {
        return Err("not a game id");
    }
    let client = client_part(said.client);
    let record = record.and_then(record_part);
    let intake = ReportIntake {
        gateway: said.gateway.clone(),
        channel: said.channel,
        reporter: said.reporter.clone(),
        kind: kind(said.kind),
        text,
        game_id: said.game_id.map(str::to_owned),
        build: said.build.clone().or_else(|| intake::build_of(said.client)),
        crash,
        client: client.as_deref().and_then(Announced::of),
        record: record.as_ref().and_then(|r| {
            let announced = Announced::of(&r.gzip)?;
            Some(AnnouncedRecord {
                complete: r.complete,
                origin: r.origin,
                bytes: announced.bytes,
                sha256: announced.sha256,
            })
        }),
    };
    if !intake.validate() {
        return Err("the report is outside what Central takes");
    }
    Ok(Filing {
        intake,
        client,
        record: record.map(|r| r.gzip),
    })
}

/// Files `filing` with Central through `bridge`, blocking: the metadata, then
/// each part it announced. Central's id for it; or why not, when the
/// metadata was not taken. A part that does not arrive leaves the report
/// filed and that part shown as awaited, with a line in the log; one Central
/// already holds (a retried upload that had landed) counts as sent.
pub(super) fn file(
    bridge: &crate::central::Bridge,
    filing: &Filing,
) -> Result<String, baylee_account::Error> {
    let id = bridge.file_report(&filing.intake)?;
    let report = intake::id_hex(&id);
    for (part, gzip) in [
        (Part::Client, &filing.client),
        (Part::Record, &filing.record),
    ] {
        let Some(gzip) = gzip else { continue };
        match bridge.put_report_part(&id, part, gzip) {
            Ok(()) | Err(baylee_account::Error::Conflict) => {}
            Err(why) => tracing::warn!(
                report,
                part = part.as_str(),
                ?why,
                "a report's part did not reach Central"
            ),
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_account::reports::Build;
    use std::io::Read as _;

    fn said<'a>(kind: ReportKind, text: &'a str, client: &'a serde_json::Value) -> Said<'a> {
        Said {
            gateway: GatewaySays {
                name: Some("Synthetic gateway".into()),
                url: None,
                version: Some("0.1.1".into()),
            },
            channel: Channel::Gateway,
            reporter: "ab12".into(),
            kind,
            text,
            game_id: None,
            client,
            build: None,
        }
    }

    #[test]
    fn what_a_client_wrote_is_brought_within_centrals_bounds() {
        let client = serde_json::json!({
            "build": {"version": "0.1.1-beta.9b\u{202E}", "commit": "not-hex"},
            "crash": {"message": "m".repeat(5_000), "location": "src/a.rs:1:2\nmore"},
            "system": {"platform": "macOS"},
        });
        let filing = prepare(
            &said(ReportKind::Crash, "It broke\r\n\u{200F}here", &client),
            None,
        )
        .unwrap();
        let intake = &filing.intake;
        assert_eq!(intake.text, "It broke\nhere");
        assert_eq!(
            intake.build,
            Some(Build {
                version: "0.1.1-beta.9b".into(),
                commit: None
            })
        );
        let crash = intake.crash.as_ref().unwrap();
        assert_eq!(crash.message.chars().count(), intake::MAX_CRASH_MESSAGE);
        assert_eq!(crash.location, "src/a.rs:1:2more");
        assert!(intake.validate());
        // the client part is the client's JSON, gzip, as announced
        let gzip = filing.client.as_ref().unwrap();
        assert_eq!(intake.client, Announced::of(gzip));
        let mut json = String::new();
        flate2::read::GzDecoder::new(gzip.as_slice())
            .read_to_string(&mut json)
            .unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&json).unwrap(),
            client
        );
    }

    #[test]
    fn a_crash_without_words_says_what_it_is() {
        let client = serde_json::json!({"crash": {"message": "\u{200E}boom\nat depth", "location": "x.rs:1"}});
        let filing = prepare(&said(ReportKind::Crash, "", &client), None).unwrap();
        assert_eq!(filing.intake.text, "boom\nat depth");
        let empty = serde_json::json!({});
        let filing = prepare(&said(ReportKind::Crash, " ", &empty), None).unwrap();
        assert_eq!(filing.intake.text, "The client crashed.");
        assert!(prepare(&said(ReportKind::Bug, " ", &empty), None).is_err());
    }

    #[test]
    fn a_crash_place_only_for_a_crash_and_no_empty_client() {
        let client = serde_json::json!({"crash": {"message": "boom", "location": "x.rs:1"}});
        let filing = prepare(&said(ReportKind::Bug, "text", &client), None).unwrap();
        assert_eq!(filing.intake.crash, None);
        let empty = serde_json::json!({});
        let filing = prepare(&said(ReportKind::Bug, "text", &empty), None).unwrap();
        assert!(filing.client.is_none() && filing.intake.client.is_none());
    }

    #[test]
    fn a_report_central_cannot_take_is_refused_or_filed_without_the_part() {
        let client = serde_json::json!({});
        assert!(prepare(&said(ReportKind::Bug, " \u{200E}\r\n", &client), None).is_err());
        let mut odd = said(ReportKind::Bug, "text", &client);
        odd.game_id = Some("not a game");
        assert!(prepare(&odd, None).is_err());
        let not_gzip = RecordBytes {
            complete: true,
            origin: RecordOrigin::Gateway,
            gzip: vec![1, 2, 3],
        };
        let filing = prepare(&said(ReportKind::Bug, "text", &client), Some(not_gzip)).unwrap();
        assert!(filing.record.is_none() && filing.intake.record.is_none());
        let large = RecordBytes {
            complete: false,
            origin: RecordOrigin::Client,
            gzip: [
                &[0x1f, 0x8b][..],
                &vec![0; intake::MAX_CLIENT_RECORD_GZIP as usize],
            ]
            .concat(),
        };
        let filing = prepare(&said(ReportKind::Bug, "text", &client), Some(large)).unwrap();
        assert!(filing.record.is_none());
        let kept = RecordBytes {
            complete: true,
            origin: RecordOrigin::Client,
            gzip: vec![0x1f, 0x8b, 8, 0],
        };
        let filing = prepare(&said(ReportKind::Bug, "text", &client), Some(kept)).unwrap();
        let announced = filing.intake.record.unwrap();
        assert_eq!(
            (announced.bytes, announced.complete, announced.origin),
            (4, true, RecordOrigin::Client)
        );
    }
}
