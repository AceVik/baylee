//! `baylee-feedback export-reports` against a real PostgreSQL: the open
//! reports outside the trash, each a line within Central's bounds, with its
//! notes and history, its client details and record as hex gzip, and a
//! last line whose count and SHA-256 cover exactly the lines before it.

#![allow(clippy::missing_docs_in_private_items)]

use std::io::{Read as _, Write as _};

use sea_orm::{ConnectionTrait, Database};
use sha2::{Digest as _, Sha256};
use uuid::Uuid;

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

fn gzip(bytes: &[u8]) -> Vec<u8> {
    let mut out = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    out.write_all(bytes).unwrap();
    out.finish().unwrap()
}

#[tokio::test]
async fn the_open_reports_go_out_as_lines_central_takes() {
    let url = std::env::var("DATABASE_URL")
        .ok()
        .filter(|u| !u.is_empty())
        .expect("DATABASE_URL is not set, and this test is about PostgreSQL");
    let schema = format!("fb_export_{}", Uuid::now_v7().simple());
    let admin = Database::connect(&url).await.expect("connecting");
    admin
        .execute_unprepared(&format!("CREATE SCHEMA \"{schema}\""))
        .await
        .expect("a schema");
    let sep = if url.contains('?') { '&' } else { '?' };
    let db = baylee_feedback::connect(&format!("{url}{sep}options=-c%20search_path%3D{schema}"), 2)
        .await
        .expect("migrating");
    let (open, record) = seed(&db).await;
    let mut out = Vec::new();
    let (count, left_out) = baylee_feedback::export::export_reports(&db, &mut out)
        .await
        .expect("the export");
    assert_eq!(
        (count, left_out),
        (2, 1),
        "the open ones, less the one with a reporter that is not hex"
    );
    let text = String::from_utf8(out).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 3);
    let trailer: serde_json::Value = serde_json::from_str(lines[2]).unwrap();
    let mut digest = Sha256::new();
    for line in &lines[..2] {
        digest.update(line.as_bytes());
        digest.update(b"\n");
    }
    let sha = hex(&digest.finalize());
    assert_eq!(
        trailer,
        serde_json::json!({"format": baylee_feedback::export::FORMAT, "count": 2, "sha256": sha})
    );

    the_lines_say(&lines, open, &record);

    drop(db);
    admin
        .execute_unprepared(&format!("DROP SCHEMA \"{schema}\" CASCADE"))
        .await
        .expect("dropping the schema");
}

/// Six reports: an open bug with a note, a history line and a record; an
/// open crash without words, whose client record hides a bomb behind a
/// small first gzip member; one resolved, one in the trash, one whose
/// reporter is not hex, and an open reported chat message, which stays.
/// The open bug's id, and its record.
async fn seed(db: &sea_orm::DatabaseConnection) -> (Uuid, Vec<u8>) {
    let record = gzip(b"{\"kind\":\"header\"}\n");
    let (open, crash, done, trashed, odd, chat) = (
        Uuid::now_v7(),
        Uuid::now_v7(),
        Uuid::now_v7(),
        Uuid::now_v7(),
        Uuid::now_v7(),
        Uuid::now_v7(),
    );
    let insert = |id: Uuid,
                  reporter: &str,
                  kind: &str,
                  status: &str,
                  text: &str,
                  client: &str,
                  extra: &str| {
        format!(
            "INSERT INTO feedback_report (id, gateway, gateway_name, gateway_url, gateway_version, reporter, kind, status, text, game_id, client, labels{}) \
             VALUES ('{id}', 'official', 'Baylee\u{202E}', NULL, '0.1.1', '{reporter}', '{kind}', '{status}', '{text}', NULL, '{client}'::jsonb, ARRAY['ui']{})",
            if extra.is_empty() {
                ""
            } else {
                ", done_at, deleted_at"
            },
            extra
        )
    };
    let reporter = "ab".repeat(32);
    for statement in [
        insert(
            open,
            &reporter,
            "bug",
            "triaged",
            "The stack\r\nwent wrong",
            r#"{"build":{"version":"0.1.1-beta.9b","commit":"3f9a1c7e21"},"os":"x"}"#,
            "",
        ),
        insert(
            crash,
            &reporter,
            "crash",
            "new",
            "",
            r#"{"crash":{"message":"boom","location":"a.rs:1"}}"#,
            "",
        ),
        insert(
            done,
            &reporter,
            "bug",
            "resolved",
            "fixed",
            "{}",
            ", now(), NULL",
        ),
        insert(
            trashed,
            &reporter,
            "bug",
            "new",
            "gone",
            "{}",
            ", NULL, now()",
        ),
        insert(odd, "not-hex", "bug", "new", "who", "{}", ""),
        insert(
            chat,
            &reporter,
            "chat",
            "new",
            "a reported message",
            "{}",
            "",
        ),
        format!(
            "UPDATE feedback_report SET record = decode('{}', 'hex'), record_complete = true, assignee = 'viktor' WHERE id = '{open}'",
            hex(&record)
        ),
        format!(
            "UPDATE feedback_report SET record = decode('{}', 'hex'), record_origin = 'client' WHERE id = '{crash}'",
            hex(&bombed())
        ),
        format!(
            "INSERT INTO feedback_note (id, report_id, author, text) VALUES ('{}', '{open}', 'viktor', 'Looks like the stack')",
            Uuid::now_v7()
        ),
        format!(
            "INSERT INTO feedback_audit (id, actor, report_id, action, detail) VALUES ('{}', 'viktor', '{open}', 'status', 'new → triaged')",
            Uuid::now_v7()
        ),
    ] {
        db.execute_unprepared(&statement)
            .await
            .unwrap_or_else(|e| panic!("{statement}: {e}"));
    }

    (open, record)
}

/// A client's record whose first gzip member is a whole small record and
/// whose second unpacks past 32 MiB.
fn bombed() -> Vec<u8> {
    let mut bombed = gzip(b"{\"kind\":\"header\",\"record\":1,\"build\":\"0.1.1\"}\n");
    bombed.extend(gzip(" ".repeat(33 * 1024 * 1024).as_bytes()));
    bombed
}

/// The two lines say what the reports held, within Central's bounds.
fn the_lines_say(lines: &[&str], open: Uuid, record: &[u8]) {
    let first: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(first["id"], open.simple().to_string());
    assert_eq!(first["text"], "The stack\nwent wrong");
    assert_eq!(
        first["gateway_says"],
        serde_json::json!({"name": "Baylee", "url": null, "version": "0.1.1"})
    );
    assert_eq!(
        first["build"],
        serde_json::json!({"version": "0.1.1-beta.9b", "commit": "3f9a1c7e21"})
    );
    assert_eq!(
        (first["status"].as_str(), first["assignee"].as_str()),
        (Some("triaged"), Some("viktor"))
    );
    assert_eq!(first["labels"], serde_json::json!(["ui"]));
    assert_eq!(first["notes"][0]["text"], "Looks like the stack");
    assert_eq!(first["audit"][0]["detail"], "new → triaged");
    let gz = |v: &serde_json::Value| {
        let text = v.as_str().unwrap();
        (0..text.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
            .collect::<Vec<u8>>()
    };
    assert_eq!(gz(&first["record"]["gzip"]), record);
    assert_eq!(first["record"]["origin"], "gateway");
    let mut client = String::new();
    flate2::read::GzDecoder::new(&gz(&first["client_gzip"])[..])
        .read_to_string(&mut client)
        .unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&client).unwrap()["os"],
        "x"
    );

    let second: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
    assert_eq!(
        second["text"], "boom",
        "a crash without words reads its message"
    );
    assert_eq!(
        second["crash"],
        serde_json::json!({"message": "boom", "location": "a.rs:1"})
    );
    assert!(
        second["record"].is_null(),
        "a client's record with a bomb behind its first member is left out"
    );
}
