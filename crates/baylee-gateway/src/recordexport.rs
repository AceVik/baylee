//! The anonymised export of game records, for training and balancing
//! (`baylee-gateway records export --out <dir>`; `docs/privacy.md`
//! §"Game records and reports", "Training and balancing use only the
//! anonymised export").
//!
//! A stored record already names nobody (seats are numbers, #315); what
//! ties it to a person is outside it: the game's id, which is the join to
//! `game_record_seat` (who sat where) and to a feedback report, and which as
//! a version-7 UUID is a time too; and the host's clock on every line (`at`). The
//! export keeps neither. Each record is written as `NNNNNN.jsonl.gz`, by a
//! count, with no id, no time and no index beside it. What stays is what
//! training and balancing read: the build, the preset (format, seed, seats
//! with their teams, decks and printings, the house's profiles), every input
//! with its seat and who answered it, the chair changes, each seat's
//! declared mind (the model a seat said it was), and the end.
//!
//! The record is read as JSON, not as the engine's types: the gateway links
//! no rules. So the shape is held loudly instead: a line of a kind this
//! export does not know refuses the record rather than passing through, a
//! seat's `Human` controller loses its account number, and the written text
//! is searched for everything that names an account seated at the game (its
//! id, email, username, display name, invite key) and refused on a find.
//! Refused records are counted, never named.

use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};

use serde_json::Value;

/// The line kinds a record holds (`baylee_gamehost::record::Line`).
const KINDS: [&str; 5] = ["header", "input", "chair", "declared_mind", "end"];

/// Fields whose value is a hash or a card's printing id: hex and uuids,
/// which a short name could occur in by chance, and which name no one.
const NOT_SEARCHED: [&str; 3] = ["kind", "hash", "scryfall_id"];

/// Below this many characters a name is not searched for: it would occur by
/// chance in anything (a username has at least three).
const SHORTEST_SEARCHED: usize = 3;

const USAGE: &str = "usage: baylee-gateway records export --out <directory>

Writes every complete game record, anonymised, as <directory>/NNNNNN.jsonl.gz:
no game id, no time, no account. Needs DATABASE_URL. The directory must be
new or empty.";

/// A record with nothing left in it that ties it to an account, or why it
/// cannot be exported. `jsonl` is the record's text (JSON Lines), `names`
/// what names the accounts seated at its game.
pub(crate) fn anonymise(jsonl: &str, names: &[String]) -> Result<String, String> {
    let names: Vec<&str> = names
        .iter()
        .map(String::as_str)
        .filter(|n| n.chars().count() >= SHORTEST_SEARCHED)
        .collect();
    let mut out = String::with_capacity(jsonl.len());
    for (n, line) in jsonl.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let mut value: Value =
            serde_json::from_str(line).map_err(|e| format!("line {}: {e}", n + 1))?;
        let object = value
            .as_object_mut()
            .ok_or_else(|| format!("line {}: not an object", n + 1))?;
        let kind = object
            .get("kind")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("line {}: no kind", n + 1))?;
        if !KINDS.contains(&kind) {
            return Err(format!("line {}: a line of kind `{kind}`", n + 1));
        }
        if kind == "header" {
            forget_accounts(object);
        }
        // The host's clock: with the order of the lines, when it was played.
        if let Some(at) = object.get_mut("at") {
            *at = Value::from(0u64);
        }
        // The line is said, never the name.
        if names.iter().any(|name| mentions(&value, name)) {
            return Err(format!("line {} names a seated account", n + 1));
        }
        out.push_str(&value.to_string());
        out.push('\n');
    }
    if out.is_empty() {
        return Err("an empty record".to_owned());
    }
    Ok(out)
}

/// A header's seats: a `Human` controller's account number becomes 0.
fn forget_accounts(header: &mut serde_json::Map<String, Value>) {
    let Some(seats) = header
        .get_mut("preset")
        .and_then(|p| p.get_mut("seats"))
        .and_then(Value::as_array_mut)
    else {
        return;
    };
    for seat in seats {
        if let Some(human) = seat
            .get_mut("controller")
            .and_then(|c| c.get_mut("Human"))
            .and_then(Value::as_object_mut)
        {
            for number in human.values_mut() {
                *number = Value::from(0u64);
            }
        }
    }
}

/// Whether a string anywhere in `value` (but under [`NOT_SEARCHED`]) holds
/// `name`, ignoring case.
fn mentions(value: &Value, name: &str) -> bool {
    let name = name.to_lowercase();
    let mut stack = vec![value];
    while let Some(value) = stack.pop() {
        match value {
            Value::String(text) => {
                if text.to_lowercase().contains(&name) {
                    return true;
                }
            }
            Value::Array(items) => stack.extend(items),
            Value::Object(fields) => stack.extend(
                fields
                    .iter()
                    .filter(|(key, _)| !NOT_SEARCHED.contains(&key.as_str()))
                    .map(|(_, v)| v),
            ),
            Value::Null | Value::Bool(_) | Value::Number(_) => {}
        }
    }
    false
}

/// A stored record (one or more gzip members, one per piece the engine
/// sent) as text.
fn ungzip(data: &[u8]) -> Result<String, String> {
    let mut text = String::new();
    flate2::read::MultiGzDecoder::new(data)
        .read_to_string(&mut text)
        .map_err(|e| format!("not a gzip of text: {e}"))?;
    Ok(text)
}

/// One gzip member.
fn gzip(text: &str) -> std::io::Result<Vec<u8>> {
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(text.as_bytes())?;
    encoder.finish()
}

/// `baylee-gateway records …`: the exit status.
pub(crate) async fn cli(args: &[String]) -> i32 {
    let out = match args {
        [command, flag, dir] if command == "export" && flag == "--out" => PathBuf::from(dir),
        [help] if help == "help" || help == "--help" => {
            println!("{USAGE}");
            return 0;
        }
        _ => {
            eprintln!("baylee-gateway records: {USAGE}");
            return 2;
        }
    };
    let Some(url) = std::env::var("DATABASE_URL").ok().filter(|u| !u.is_empty()) else {
        eprintln!("baylee-gateway records: DATABASE_URL is not set");
        return 1;
    };
    let db = match baylee_db::connect(&url, 1).await {
        Ok(db) => db,
        Err(e) => {
            eprintln!("baylee-gateway records: {e:#}");
            return 1;
        }
    };
    match export(&db, &out).await {
        Ok((written, refused)) => {
            println!("exported {written} records, refused {refused}");
            0
        }
        Err(why) => {
            eprintln!("baylee-gateway records: {why}");
            1
        }
    }
}

/// Writes every complete record into `out`: how many, and how many were
/// refused.
async fn export(db: &sea_orm::DatabaseConnection, out: &Path) -> Result<(usize, usize), String> {
    std::fs::create_dir_all(out).map_err(|e| format!("cannot make {}: {e}", out.display()))?;
    let leftover = std::fs::read_dir(out)
        .map_err(|e| format!("cannot read {}: {e}", out.display()))?
        .next()
        .is_some();
    if leftover {
        // An export beside another would number over it, or be mistaken for
        // one with the other's records in it.
        return Err(format!("{} is not empty", out.display()));
    }
    let ids = baylee_db::records::complete_ids(db)
        .await
        .map_err(|e| format!("the records: {e}"))?;
    let (mut written, mut refused) = (0usize, 0usize);
    for id in ids {
        let Some(record) = baylee_db::records::for_export(db, &id)
            .await
            .map_err(|e| format!("a record: {e}"))?
        else {
            continue;
        };
        let names = baylee_db::records::seated_identities(db, &id)
            .await
            .map_err(|e| format!("a record's seats: {e}"))?;
        let anonymous = ungzip(&record.data).and_then(|text| anonymise(&text, &names));
        let Ok(text) = anonymous else {
            refused += 1;
            continue;
        };
        written += 1;
        let path = out.join(format!("{written:06}.jsonl.gz"));
        let bytes = gzip(&text).map_err(|e| format!("compressing: {e}"))?;
        std::fs::write(&path, bytes)
            .map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    }
    Ok((written, refused))
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEADER: &str = r#"{"kind":"header","record":1,"build":"b","hash":"abc","preset":{"format":"Standard","seed":7,"seats":[{"controller":{"Human":{"user_id":42}},"team":1,"deck":[{"card":3,"print":0}]},{"controller":{"Ai":"Sharp"},"team":2,"deck":[]}]}}"#;

    fn record(lines: &[&str]) -> String {
        let mut text = String::new();
        for line in lines {
            text.push_str(line);
            text.push('\n');
        }
        text
    }

    fn lines(text: &str) -> Vec<Value> {
        text.lines()
            .map(|l| serde_json::from_str(l).expect("json"))
            .collect()
    }

    /// The account number a `Human` seat carries, and the host's clock,
    /// do not survive; seats, teams, decks, minds and the end do.
    #[test]
    fn an_account_number_and_the_clock_are_dropped_and_the_game_stays() {
        let text = record(&[
            HEADER,
            r#"{"kind":"input","n":1,"at":1791000000000,"seat":0,"by":"seat","action":"Concede","hash":"h"}"#,
            r#"{"kind":"declared_mind","n":2,"at":1791000000001,"seat":1,"mind":{"kind":"llm_api","provider":"anthropic","model":"claude-sonnet-5-5"}}"#,
            r#"{"kind":"end","n":3,"at":1791000000002,"winners":[1],"reason":"concede"}"#,
        ]);
        let out = lines(&anonymise(&text, &[]).expect("exported"));
        let seats = &out[0]["preset"]["seats"];
        assert_eq!(seats[0]["controller"]["Human"]["user_id"], 0, "{seats}");
        assert_eq!(seats[0]["team"], 1);
        assert_eq!(seats[0]["deck"][0]["card"], 3);
        assert_eq!(seats[1]["controller"]["Ai"], "Sharp");
        for line in &out[1..] {
            assert_eq!(line["at"], 0, "{line}");
        }
        assert_eq!(out[1]["seat"], 0);
        assert_eq!(out[2]["mind"]["model"], "claude-sonnet-5-5");
        assert_eq!(out[3]["winners"][0], 1);
        assert!(!anonymise(&text, &[]).unwrap().contains("1791000000"));
    }

    /// A kind this export does not know refuses the record instead of
    /// passing through: a positive list goes silent on a new line kind.
    #[test]
    fn a_line_of_an_unknown_kind_refuses_the_record() {
        let text = record(&[HEADER, r#"{"kind":"chat","seat":0,"text":"hi"}"#]);
        assert!(anonymise(&text, &[]).is_err());
        assert!(anonymise("not json\n", &[]).is_err());
        assert!(anonymise("", &[]).is_err());
    }

    /// A seated account's name, id or email anywhere in the text refuses
    /// the record, in any case; a name too short to search for, and a hash
    /// a name happens to occur in, do not.
    #[test]
    fn a_seated_accounts_name_anywhere_refuses_the_record() {
        let mind = r#"{"kind":"declared_mind","n":1,"at":0,"seat":0,"mind":{"kind":"llm_cli","model":"Rhea-bot"}}"#;
        let text = record(&[HEADER, mind]);
        let named = |name: &str| anonymise(&text, &[name.to_owned()]);
        assert!(named("rhea").is_err(), "a display name in a mind");
        assert!(named("RHEA").is_err());
        assert!(named("Sharp").is_err(), "a name in a seat's profile");
        assert!(named("rh").is_ok(), "two characters are searched nowhere");
        assert!(named("abc").is_ok(), "a header's hash is not searched");
        assert!(named("zoe").is_ok());
    }

    #[test]
    fn a_stored_record_of_several_pieces_reads_whole() {
        let mut stored = gzip("{\"kind\":\"header\"}\n").unwrap();
        stored.extend(gzip("{\"kind\":\"end\"}\n").unwrap());
        assert_eq!(
            ungzip(&stored).unwrap(),
            "{\"kind\":\"header\"}\n{\"kind\":\"end\"}\n"
        );
    }
}
