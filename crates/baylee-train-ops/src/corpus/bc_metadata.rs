//! Native source-record identities and overlap admission for Torch BC mixtures.
use crate::files;
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write,
    fs::File,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

fn python_float(value: &serde_json::Number) -> String {
    let text = value.to_string();
    let (sign, text) = text
        .strip_prefix('-')
        .map_or(("", text.as_str()), |s| ("-", s));
    let (mantissa, exponent) = text.split_once('e').map_or((text, 0), |(m, e)| {
        (m, e.parse::<i32>().expect("JSON float exponent"))
    });
    let decimal = mantissa.find('.').unwrap_or(mantissa.len());
    let raw = mantissa.replace('.', "");
    let leading = raw.bytes().take_while(|b| *b == b'0').count();
    let digits = raw[leading..].trim_end_matches('0');
    if digits.is_empty() {
        return format!("{sign}0.0");
    }
    let power = exponent + i32::try_from(decimal).expect("short float")
        - i32::try_from(leading).expect("short float")
        - 1;
    if !(-4..16).contains(&power) {
        let fraction = if digits.len() == 1 {
            String::new()
        } else {
            format!(".{}", &digits[1..])
        };
        return format!("{sign}{}{fraction}e{power:+03}", &digits[..1]);
    }
    let position = power + 1;
    if position <= 0 {
        return format!(
            "{sign}0.{}{}",
            "0".repeat(usize::try_from(-position).expect("bounded exponent")),
            digits
        );
    }
    let position = usize::try_from(position).expect("positive exponent");
    if position >= digits.len() {
        format!("{sign}{digits}{}.0", "0".repeat(position - digits.len()))
    } else {
        format!("{sign}{}.{}", &digits[..position], &digits[position..])
    }
}
fn python_json(value: &Value) -> Result<String> {
    python_json_style(value, false)
}
fn python_json_style(value: &Value, spaced: bool) -> Result<String> {
    let separator = if spaced { ", " } else { "," };
    Ok(match value {
        Value::Array(rows) => format!(
            "[{}]",
            rows.iter()
                .map(|value| python_json_style(value, spaced))
                .collect::<Result<Vec<_>>>()?
                .join(separator)
        ),
        Value::Object(fields) => {
            let fields: BTreeMap<_, _> = fields.iter().collect();
            format!(
                "{{{}}}",
                fields
                    .iter()
                    .map(|(key, value)| Ok(format!(
                        "{}{}{}",
                        python_json_style(&json!(key), spaced)?,
                        if spaced { ": " } else { ":" },
                        python_json_style(value, spaced)?
                    )))
                    .collect::<Result<Vec<_>>>()?
                    .join(separator)
            )
        }
        Value::String(text) => {
            let escaped = serde_json::to_string(text)?;
            let mut ascii = String::new();
            for c in escaped.chars() {
                if (' '..='~').contains(&c) {
                    ascii.push(c);
                } else {
                    let mut units = [0u16; 2];
                    for unit in c.encode_utf16(&mut units) {
                        write!(ascii, "\\u{unit:04x}")?;
                    }
                }
            }
            ascii
        }
        Value::Number(number) if number.is_f64() => python_float(number),
        _ => serde_json::to_string(value)?,
    })
}
/// Preserve Python's compact sorted ASCII JSON SHA256 used by mixture contracts.
///
/// # Errors
/// Returns serialization errors for malformed JSON values.
pub fn digest(value: &Value) -> Result<String> {
    Ok(files::hex(&Sha256::digest(python_json(value)?.as_bytes())))
}
/// Preserve default-spaced sorted ASCII JSON cache keys.
/// # Errors
/// Returns JSON serialization errors.
pub fn spaced_digest(value: &Value) -> Result<String> {
    Ok(files::hex(&Sha256::digest(
        python_json_style(value, true)?.as_bytes(),
    )))
}
pub(super) fn resolve(path: &Path) -> Result<PathBuf> {
    if let Ok(path) = path.canonicalize() {
        return Ok(path);
    }
    let path = std::path::absolute(path)?;
    let mut head = path.as_path();
    let mut tail = Vec::new();
    while !head.exists() {
        tail.push(head.file_name().context("invalid source path")?.to_owned());
        head = head.parent().context("invalid source path")?;
    }
    let mut result = head.canonicalize()?;
    for part in tail.into_iter().rev() {
        result.push(part);
    }
    Ok(result)
}
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Record {
    path: String,
    offset: u64,
    length: u64,
}
impl Record {
    fn from_row(root: &Path, row: &Value) -> Result<Self> {
        let shard = row["shard"].as_str().context("missing source shard")?;
        Ok(Self {
            path: resolve(&root.join("records").join(shard))?
                .to_str()
                .context("invalid source shard path")?
                .to_owned(),
            offset: row["offset"].as_u64().context("invalid record offset")?,
            length: row["len"].as_u64().context("invalid record length")?,
        })
    }
    fn value(&self) -> Value {
        json!([self.path, self.offset, self.length])
    }
}
fn entries(root: &Path, meta: &Value) -> Result<Vec<Value>> {
    let values = if !meta["package_sources"].is_null() {
        meta["package_sources"].clone()
    } else if root.join("source_markers.json").exists() {
        files::read_json(&root.join("source_markers.json"))?
    } else {
        let mut paths = std::fs::read_dir(root.join("ready"))
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "json"))
            .collect::<Vec<_>>();
        paths.sort();
        json!(
            paths
                .iter()
                .map(|p| files::read_json(p))
                .collect::<Result<Vec<_>>>()?
        )
    };
    values.as_array().cloned().context("invalid source markers")
}
fn source_index(
    index: &Path,
    receipts: &mut BTreeMap<String, String>,
) -> Result<BTreeMap<Record, (u64, u64)>> {
    let root = index.parent().context("missing source index root")?;
    let run = root.join("run.json");
    let mut run_seats = None;
    if run.exists() {
        let raw = std::fs::read(&run)?;
        let meta: Value = serde_json::from_slice(&raw)?;
        if !meta["seats"].is_null() {
            run_seats = Some(
                meta["seats"]
                    .as_u64()
                    .context("Source index lacks complete table seat metadata")?,
            );
        }
        receipts.insert(
            run.to_str().context("invalid run path")?.to_owned(),
            files::hex(&Sha256::digest(raw)),
        );
    }
    let mut result = BTreeMap::new();
    let mut hash = Sha256::new();
    let mut stream = BufReader::new(File::open(index)?);
    let mut line = Vec::new();
    while stream.read_until(b'\n', &mut line)? > 0 {
        hash.update(&line);
        let row: Value = serde_json::from_slice(&line)?;
        let key = Record::from_row(root, &row)?;
        let seats = row["decks"]
            .as_array()
            .map(|d| d.len() as u64)
            .or(run_seats)
            .context("Source index lacks complete table seat metadata")?;
        ensure!(
            (2..=8).contains(&seats),
            "Source index lacks complete table seat metadata"
        );
        ensure!(
            run_seats.is_none_or(|s| s == seats),
            "Source index and run seat metadata differ"
        );
        ensure!(
            result
                .insert(
                    key,
                    (row["seed"].as_u64().context("missing source seed")?, seats)
                )
                .is_none(),
            "Duplicate source-record metadata"
        );
        line.clear();
    }
    receipts.insert(
        index
            .to_str()
            .context("invalid source index path")?
            .to_owned(),
        files::hex(&hash.finalize()),
    );
    Ok(result)
}
/// Bind observed Torch trajectories to exact source records, whole tables and receipts.
///
/// # Errors
/// Rejects duplicate, missing, unverified or incomplete source-game identities.
pub fn game_metadata(
    root: &Path,
    meta: &Value,
    split: &str,
    observed: &[(u64, u64)],
) -> Result<Value> {
    let root = root.canonicalize()?;
    let namespace = meta
        .get("source_namespace")
        .cloned()
        .unwrap_or_else(|| json!({"dataset":root}));
    let mut observed_games: BTreeMap<u64, BTreeSet<u64>> = BTreeMap::new();
    for (game, seat) in observed {
        observed_games.entry(*game).or_default().insert(*seat);
    }
    let mut hashes = BTreeMap::new();
    for key in ["shadow_bc_sources", "llm_bc_sources"] {
        if let Some(sources) = meta[key].as_array() {
            for source in sources {
                hashes.insert(
                    source["converted_game_id"]
                        .as_u64()
                        .context("invalid converted game")?,
                    source["record_sha256"].clone(),
                );
            }
        }
    }
    let mut indices = BTreeMap::new();
    let mut games = BTreeMap::new();
    let mut receipts = BTreeMap::new();
    let mut seen = BTreeSet::new();
    for entry in entries(&root, meta)?
        .into_iter()
        .filter(|e| e["split"] == split)
    {
        let game = entry["game_id"]
            .as_u64()
            .context("invalid converted game marker")?;
        let source = Path::new(
            entry["source_shard"]
                .as_str()
                .context("missing source marker shard")?,
        );
        ensure!(
            source.is_absolute() && entry["replay_hashes_checked"] == true,
            "BC mixture requires verified absolute source markers"
        );
        let source = resolve(source)?;
        let record = Record {
            path: source.to_str().context("invalid source shard")?.to_owned(),
            offset: entry["source_offset"]
                .as_u64()
                .context("invalid source offset")?,
            length: entry["source_len"]
                .as_u64()
                .context("invalid source length")?,
        };
        ensure!(
            seen.insert(record.clone()),
            "Duplicate converted source record"
        );
        let index = source
            .parent()
            .and_then(Path::parent)
            .context("missing source root")?
            .join("games.jsonl");
        if !indices.contains_key(&index) {
            indices.insert(index.clone(), source_index(&index, &mut receipts)?);
        }
        let (seed, seats) = indices[&index]
            .get(&record)
            .context("Source record missing from game index")?;
        ensure!(
            observed_games.get(&game) == Some(&(0..*seats).collect()),
            "BC mixture requires all source game seats"
        );
        let value = json!({"record":record.value(),"record_sha256":hashes.get(&game),"table":digest(&json!({"namespace":namespace,"seed":seed}))?,"source_table":digest(&json!({"run":index.parent(),"seed":seed}))?});
        ensure!(
            games.insert(game.to_string(), value).is_none(),
            "Duplicate converted game marker"
        );
    }
    ensure!(
        games
            .keys()
            .filter_map(|g| g.parse::<u64>().ok())
            .collect::<BTreeSet<_>>()
            == observed_games.keys().copied().collect(),
        "Source markers do not cover exact dataset split"
    );
    Ok(json!({"games":games,"receipts":receipts}))
}
/// Reject any repeated source record, raw record hash, namespaced table or source table.
///
/// # Errors
/// Returns an error for overlap or malformed identity maps.
pub fn reject_overlap(left: &Value, right: &Value) -> Result<()> {
    let left = left.as_object().context("invalid left source metadata")?;
    let right = right.as_object().context("invalid right source metadata")?;
    let mut records = BTreeSet::new();
    let mut hashes = BTreeSet::new();
    let mut tables = BTreeSet::new();
    let mut sources = BTreeSet::new();
    for value in left.values() {
        records.insert(serde_json::to_string(&value["record"])?);
        if let Some(hash) = value["record_sha256"].as_str().filter(|s| !s.is_empty()) {
            hashes.insert(hash);
        }
        tables.insert(value["table"].as_str().context("missing table identity")?);
        sources.insert(
            value["source_table"]
                .as_str()
                .context("missing source table identity")?,
        );
    }
    for value in right.values() {
        ensure!(
            !records.contains(&serde_json::to_string(&value["record"])?)
                && !tables.contains(value["table"].as_str().context("missing table identity")?)
                && !sources.contains(
                    value["source_table"]
                        .as_str()
                        .context("missing source table identity")?
                )
                && !value["record_sha256"]
                    .as_str()
                    .is_some_and(|h| hashes.contains(h)),
            "BC mixture source game/table overlaps another source or validation"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn canonical_json_preserves_ascii_escaping_and_sorted_contract_keys() -> Result<()> {
        let value = json!({"z":"ß😀\u{7f}","a":[2,null,true]});
        assert_eq!(
            python_json(&value)?,
            r#"{"a":[2,null,true],"z":"\u00df\ud83d\ude00\u007f"}"#
        );
        assert_eq!(
            python_json_style(&json!({"b":[1,2],"a":"ß"}), true)?,
            r#"{"a": "\u00df", "b": [1, 2]}"#
        );
        assert_eq!(
            python_json(&json!([1e-5, 1e-4, 1e16, 1.0, -0.0, 0.25]))?,
            "[1e-05,0.0001,1e+16,1.0,-0.0,0.25]"
        );
        Ok(())
    }
    #[test]
    fn metadata_seals_complete_sources_and_rejects_missing_seats_or_overlaps() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let root = directory.path();
        let source = root.join("source");
        std::fs::create_dir_all(source.join("records"))?;
        let shard = source.join("records/a.gz");
        let index = source.join("games.jsonl");
        std::fs::write(
            &index,
            format!(
                "{}\n",
                json!({"shard":"a.gz","offset":0,"len":10,"seed":7,"decks":["a","b"]})
            ),
        )?;
        let meta = json!({"source_namespace":"néural","package_sources":[{"game_id":0,"split":"train","source_shard":shard,"source_offset":0,"source_len":10,"replay_hashes_checked":true}],"llm_bc_sources":[{"converted_game_id":0,"record_sha256":"record-hash"}]});
        let result = game_metadata(root, &meta, "train", &[(0, 0), (0, 1)])?;
        assert_eq!(
            // Receipts are keyed by canonical path (`resolve`); a temporary
            // directory is a symlink on macOS.
            result["receipts"][index.canonicalize()?.to_str().unwrap()],
            files::sha256(&index)?
        );
        assert_eq!(result["games"]["0"]["record_sha256"], "record-hash");
        assert!(
            game_metadata(root, &meta, "train", &[(0, 0)])
                .unwrap_err()
                .to_string()
                .contains("all source game seats")
        );
        assert!(reject_overlap(&result["games"], &result["games"]).is_err());
        let mut other = result["games"].clone();
        other["0"]["record"] = json!(["/different", 0, 10]);
        other["0"]["table"] = json!("different-table");
        other["0"]["source_table"] = json!("different-source-table");
        assert!(reject_overlap(&result["games"], &other).is_err());
        other["0"]["record_sha256"] = Value::Null;
        reject_overlap(&result["games"], &other)?;
        let mut duplicated = meta.clone();
        let row = duplicated["package_sources"][0].clone();
        duplicated["package_sources"]
            .as_array_mut()
            .unwrap()
            .push(row);
        assert!(game_metadata(root, &duplicated, "train", &[(0, 0), (0, 1)]).is_err());
        Ok(())
    }
}
