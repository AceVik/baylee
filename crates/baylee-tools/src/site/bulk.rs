//! Bounded-memory Scryfall printing selection and the picture permission rule.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, ensure};
use serde_json::Value;

pub(super) const PICTURED: [&str; 5] = [
    "implemented",
    "generated",
    "partial",
    "unimplemented",
    "never",
];
pub(super) const UB_SETS: [&str; 17] = [
    "ltr", "ltc", "who", "pip", "acr", "fin", "fic", "40k", "bot", "rex", "msh", "msc", "spe",
    "tla", "tle", "ptla", "clu",
];
const UA: &str = "baylee-site-card-status/1 (+https://github.com/AceVik/baylee)";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Fact {
    pub stamp: Option<String>,
    pub uri: Option<String>,
    pub artist: Option<String>,
    pub set: Option<String>,
}
pub(super) type Facts = BTreeMap<String, Fact>;
pub(super) type Printings = BTreeMap<String, BTreeMap<String, String>>;

pub(super) fn string<'a>(c: &'a Value, key: &str) -> Option<&'a str> {
    c.get(key)?.as_str().filter(|s| !s.is_empty())
}

pub(super) fn valid_uuid(s: &str) -> bool {
    s.len() == 36
        && s.bytes().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b == b'-'
            } else {
                b.is_ascii_digit() || (b'a'..=b'f').contains(&b)
            }
        })
}

pub(super) fn valid_code(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
}

pub(super) fn pictured<'a>(
    status: &str,
    listed: &str,
    img: Option<&'a str>,
    facts: &Facts,
) -> Option<&'a str> {
    let img = img.filter(|s| !s.is_empty())?;
    if !PICTURED.contains(&status) || UB_SETS.contains(&listed) {
        return None;
    }
    if let Some(f) = facts.get(img)
        && (f.stamp.as_deref() == Some("triangle")
            || UB_SETS.contains(&f.set.as_deref().unwrap_or(listed)))
    {
        return None;
    }
    Some(img)
}

pub(super) fn fact(c: &Value) -> Fact {
    let first = c
        .get("card_faces")
        .and_then(Value::as_array)
        .and_then(|f| f.first());
    let uri = string(c, "scryfall_uri")
        .and_then(|s| s.split('?').next())
        .filter(|s| s.starts_with("https://scryfall.com/card/"));
    Fact {
        stamp: string(c, "security_stamp").map(str::to_owned),
        uri: uri.map(str::to_owned),
        artist: string(c, "artist")
            .or_else(|| first.and_then(|f| string(f, "artist")))
            .map(str::to_owned),
        set: string(c, "set").map(str::to_owned),
    }
}

pub(super) fn printing_facts(path: &Path) -> Facts {
    let mut facts = Facts::new();
    let Ok(entries) = fs::read_dir(path) else {
        return facts;
    };
    let mut paths: Vec<_> = entries
        .filter_map(std::result::Result::ok)
        .map(|e| e.path())
        .collect();
    paths.sort();
    for path in paths {
        if path.extension().is_none_or(|s| s != "json") {
            continue;
        }
        let Ok(file) = File::open(path) else {
            continue;
        };
        let Ok(c) = serde_json::from_reader::<_, Value>(BufReader::new(file)) else {
            continue;
        };
        if let Some(id) = string(&c, "id").filter(|id| valid_uuid(id)) {
            facts.insert(id.into(), fact(&c));
        }
    }
    facts
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Number {
    // Decimal strings, normalised and compared by length, avoid overflow on malicious input.
    digits: usize,
    value: String,
    suffix: String,
}
fn num_key(s: &str) -> Number {
    let split = s.bytes().take_while(u8::is_ascii_digit).count();
    let (number, suffix) = if split == 0 {
        ("1073741824", s)
    } else {
        (&s[..split], &s[split..])
    };
    let number = number.trim_start_matches('0');
    let number = if number.is_empty() { "0" } else { number };
    Number {
        digits: number.len(),
        value: number.into(),
        suffix: suffix.into(),
    }
}
#[derive(Clone)]
struct Candidate {
    key: (bool, bool, bool, Number),
    id: String,
    fact: Fact,
}

pub(super) fn bulk_printings(
    path: &Path,
    wanted: &BTreeSet<String>,
    facts: Option<&mut Facts>,
) -> Result<Printings> {
    let file = File::open(path).context("open Scryfall bulk input")?;
    let reader: Box<dyn Read> = if path.extension().is_some_and(|s| s == "gz") {
        Box::new(flate2::read::MultiGzDecoder::new(file))
    } else {
        Box::new(file)
    };
    choose(BufReader::new(reader), wanted, facts)
}

pub(super) fn choose(
    reader: impl BufRead,
    wanted: &BTreeSet<String>,
    facts: Option<&mut Facts>,
) -> Result<Printings> {
    let mut best: BTreeMap<String, BTreeMap<String, Candidate>> = BTreeMap::new();
    for (line_number, line) in reader.lines().enumerate() {
        let line = line?;
        let line = line.trim().trim_end_matches(',');
        if !line.starts_with('{') {
            continue;
        }
        let c: Value = serde_json::from_str(line)
            .with_context(|| format!("invalid bulk JSON at line {}", line_number + 1))?;
        let first = c
            .get("card_faces")
            .and_then(Value::as_array)
            .and_then(|f| f.first());
        let oid = string(&c, "oracle_id").or_else(|| first.and_then(|f| string(f, "oracle_id")));
        let Some(oid) = oid.filter(|o| wanted.contains(*o)) else {
            continue;
        };
        if c.get("lang").is_some_and(|l| l != "en")
            || matches!(string(&c, "image_status"), Some("missing" | "placeholder"))
        {
            continue;
        }
        let faces_image = c
            .get("card_faces")
            .and_then(Value::as_array)
            .is_some_and(|faces| faces.iter().any(|f| f.get("image_uris").is_some()));
        if c.get("image_uris").is_none() && !faces_image {
            continue;
        }
        let id = string(&c, "id")
            .filter(|id| valid_uuid(id))
            .context("bulk printing has invalid id")?;
        let set = string(&c, "set")
            .filter(|s| valid_code(s))
            .context("bulk printing has invalid set code")?;
        let candidate = Candidate {
            key: (
                string(&c, "security_stamp") == Some("triangle"),
                c["promo"].as_bool().unwrap_or(false),
                c["digital"].as_bool().unwrap_or(false),
                num_key(string(&c, "collector_number").unwrap_or("")),
            ),
            id: id.into(),
            fact: fact(&c),
        };
        let slots = best.entry(oid.into()).or_default();
        for slot in [set, "*"] {
            if slots.get(slot).is_none_or(|old| candidate.key < old.key) {
                slots.insert(slot.into(), candidate.clone());
            }
        }
    }
    let mut facts = facts;
    Ok(best
        .into_iter()
        .map(|(oid, slots)| {
            let ids = slots
                .into_iter()
                .map(|(set, candidate)| {
                    if let Some(facts) = facts.as_deref_mut() {
                        facts.insert(candidate.id.clone(), candidate.fact);
                    }
                    (set, candidate.id)
                })
                .collect();
            (oid, ids)
        })
        .collect())
}

fn agent(seconds: u64) -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(seconds)))
        .build()
        .into()
}

pub(super) fn sets(cache: &Path, fetch: bool) -> Result<BTreeMap<String, Value>> {
    if !cache.exists() && fetch {
        let response = agent(60)
            .get("https://api.scryfall.com/sets")
            .header("User-Agent", UA)
            .header("Accept", "application/json")
            .call()?;
        if let Some(parent) = cache.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut temp = tempfile::NamedTempFile::new_in(cache.parent().unwrap_or(Path::new(".")))?;
        std::io::copy(&mut response.into_body().into_reader(), &mut temp)?;
        temp.persist(cache)?;
    }
    if !cache.exists() {
        return Ok(BTreeMap::new());
    }
    let doc: Value = serde_json::from_reader(BufReader::new(File::open(cache)?))?;
    let data = doc["data"]
        .as_array()
        .context("set cache has no data array")?;
    data.iter()
        .map(|s| {
            let code = string(s, "code")
                .filter(|s| valid_code(s))
                .context("invalid cached set code")?;
            Ok((code.into(), s.clone()))
        })
        .collect()
}

pub(super) fn fetch_bulk(dir: &Path) -> Result<PathBuf> {
    let mut response = agent(60)
        .get("https://api.scryfall.com/bulk-data/default-cards")
        .header("User-Agent", UA)
        .header("Accept", "application/json")
        .call()?;
    let meta: Value = response.body_mut().read_json()?;
    let uri = string(&meta, "jsonl_download_uri")
        .or_else(|| string(&meta, "download_uri"))
        .context("bulk metadata missing download URI")?;
    ensure!(uri.starts_with("https://"), "bulk download must use HTTPS");
    let filename = uri
        .split('?')
        .next()
        .unwrap_or(uri)
        .rsplit('/')
        .next()
        .filter(|s| !s.is_empty() && *s != "." && *s != "..")
        .context("bulk download has no filename")?;
    let path = dir.join(filename);
    if !path.exists() {
        fs::create_dir_all(dir)?;
        let response = agent(600)
            .get(uri)
            .header("User-Agent", UA)
            .header("Accept-Encoding", "identity")
            .call()?;
        let mut temp = tempfile::NamedTempFile::new_in(dir)?;
        std::io::copy(&mut response.into_body().into_reader(), &mut temp)?;
        temp.persist(&path)?;
    }
    Ok(path)
}
