//! Version-2 Impressum obfuscation. Never log source data or underlying parser errors.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

use anyhow::{Result, anyhow, bail, ensure};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

pub(super) const MAGIC: &[u8; 4] = b"BYIM";
pub(super) const VERSION: u8 = 2;
pub(super) const NONCE_LEN: usize = 16;
pub(super) const MAX_LEN: usize = 64;
pub(super) const SCRIPT_KEY: [u8; 16] = [
    0x6b, 0x1f, 0x0c, 0x9e, 0x52, 0xd8, 0x4a, 0x37, 0xb0, 0xe1, 0xf6, 0xa2, 0xc9, 0xd3, 0x5e, 0x48,
];
pub(super) const FIELDS: [&str; 8] = [
    "name",
    "street",
    "postcode",
    "city",
    "country_de",
    "country_en",
    "email",
    "phone",
];
pub(super) type Fields = BTreeMap<String, String>;
const MAX_BLOB: usize = 64 * 1024;

pub(super) fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(char::from(DIGITS[usize::from(b >> 4)]));
        out.push(char::from(DIGITS[usize::from(b & 15)]));
    }
    out
}

pub(super) fn xor(data: &[u8], nonce: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len());
    for (counter, chunk) in data.chunks(32).enumerate() {
        let mut hash = Sha256::new();
        hash.update(SCRIPT_KEY);
        hash.update(nonce);
        hash.update((counter as u32).to_be_bytes());
        out.extend(chunk.iter().zip(hash.finalize()).map(|(a, b)| a ^ b));
    }
    out
}

fn clean(fields: &Value) -> Result<Fields> {
    ensure!(fields.is_object(), "private JSON must be an object");
    let mut out = Fields::new();
    for key in FIELDS {
        let value = fields
            .get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| anyhow!("missing or empty field: {key}"))?;
        ensure!(
            value.chars().count() <= MAX_LEN,
            "field longer than 64 characters: {key}"
        );
        out.insert(key.into(), value.into());
    }
    Ok(out)
}

pub(super) fn needles(fields: &Fields) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for value in fields.values() {
        out.insert(value.clone());
        out.extend(
            value
                .split_whitespace()
                .filter(|w| w.chars().count() >= 3)
                .map(str::to_owned),
        );
    }
    out.insert(fields["phone"].chars().filter(|c| c.is_numeric()).collect());
    let (local, domain) = fields["email"]
        .split_once('@')
        .unwrap_or((&fields["email"], ""));
    out.insert(local.into());
    out.insert(domain.into());
    out.retain(|s| !s.is_empty());
    out
}

pub(super) fn readable(blob: &str, fields: &Fields) -> bool {
    let Ok(raw) = URL_SAFE_NO_PAD.decode(blob) else {
        return true;
    };
    let lower = blob.to_lowercase();
    let hexed = hex(&raw);
    needles(fields).iter().any(|n| {
        lower.contains(&n.to_lowercase())
            || raw.windows(n.len()).any(|w| w == n.as_bytes())
            || hexed.contains(&hex(n.as_bytes()))
    })
}

fn random(buffer: &mut [u8]) -> Result<()> {
    getrandom::fill(buffer).map_err(|_| anyhow!("system randomness unavailable"))
}

fn shuffle<T>(items: &mut [T]) -> Result<()> {
    for i in (1..items.len()).rev() {
        let bound = u64::try_from(i + 1)?;
        // Rejection sampling keeps every permutation equally likely.
        let threshold = bound.wrapping_neg() % bound;
        let value = loop {
            let mut bytes = [0; 8];
            random(&mut bytes)?;
            let value = u64::from_le_bytes(bytes);
            if value >= threshold {
                break value;
            }
        };
        items.swap(i, usize::try_from(value % bound)?);
    }
    Ok(())
}

pub(super) fn encode(fields: &Value, nonce: Option<&[u8]>) -> Result<String> {
    encode_checked(fields, nonce, readable)
}

pub(super) fn encode_checked(
    fields: &Value,
    nonce: Option<&[u8]>,
    mut reject: impl FnMut(&str, &Fields) -> bool,
) -> Result<String> {
    let fields = clean(fields)?;
    if let Some(nonce) = nonce {
        ensure!(nonce.len() == NONCE_LEN, "nonce must be 16 bytes");
    }
    for _ in 0..64 {
        let mut scattered = BTreeMap::new();
        for key in FIELDS {
            let mut pairs: Vec<_> = fields[key]
                .chars()
                .enumerate()
                .map(|(i, c)| [i as u32, u32::from(c)])
                .collect();
            shuffle(&mut pairs)?;
            scattered.insert(key, pairs);
        }
        let plain = serde_json::to_vec(&json!({"v": VERSION, "fields": scattered}))
            .map_err(|_| anyhow!("cannot encode Impressum document"))?;
        let mut random_nonce = [0; NONCE_LEN];
        let nonce = if let Some(nonce) = nonce {
            nonce
        } else {
            random(&mut random_nonce)?;
            &random_nonce
        };
        let mut raw = Vec::with_capacity(5 + NONCE_LEN + plain.len());
        raw.extend(MAGIC);
        raw.push(VERSION);
        raw.extend(nonce);
        raw.extend(xor(&plain, nonce));
        let blob = URL_SAFE_NO_PAD.encode(raw);
        if !reject(&blob, &fields) {
            return Ok(blob);
        }
    }
    bail!("every blob tried showed part of the data; try again")
}

#[cfg(test)]
pub(super) fn decode(blob: &str) -> Result<Fields> {
    let blob = blob.trim();
    ensure!(blob.len() <= MAX_BLOB, "Impressum blob too large");
    let raw = URL_SAFE_NO_PAD
        .decode(blob)
        .map_err(|_| anyhow!("invalid Impressum base64"))?;
    let head = 5 + NONCE_LEN;
    ensure!(
        raw.len() > head && raw[..4] == MAGIC[..] && raw[4] == VERSION,
        "not an Impressum blob of this version"
    );
    let plain = xor(&raw[head..], &raw[5..head]);
    let doc: Value =
        serde_json::from_slice(&plain).map_err(|_| anyhow!("invalid Impressum document"))?;
    ensure!(
        doc["v"].as_u64() == Some(u64::from(VERSION)),
        "unexpected Impressum document version"
    );
    let fields = doc["fields"]
        .as_object()
        .ok_or_else(|| anyhow!("missing Impressum fields"))?;
    ensure!(
        fields.len() == FIELDS.len() && FIELDS.iter().all(|key| fields.contains_key(*key)),
        "unexpected Impressum fields"
    );
    let mut out = Fields::new();
    for key in FIELDS {
        let pairs = fields[key]
            .as_array()
            .ok_or_else(|| anyhow!("invalid Impressum character list"))?;
        ensure!(
            !pairs.is_empty() && pairs.len() <= MAX_LEN,
            "invalid Impressum field length"
        );
        let mut chars = vec![None; pairs.len()];
        for pair in pairs {
            let pair = pair
                .as_array()
                .filter(|p| p.len() == 2)
                .ok_or_else(|| anyhow!("invalid Impressum character pair"))?;
            let i = pair[0]
                .as_u64()
                .and_then(|v| usize::try_from(v).ok())
                .filter(|i| *i < chars.len())
                .ok_or_else(|| anyhow!("invalid Impressum character position"))?;
            let cp = pair[1]
                .as_u64()
                .and_then(|v| u32::try_from(v).ok())
                .and_then(char::from_u32)
                .ok_or_else(|| anyhow!("invalid Impressum code point"))?;
            ensure!(
                chars[i].replace(cp).is_none(),
                "duplicate Impressum character position"
            );
        }
        let text: String = chars
            .into_iter()
            .collect::<Option<String>>()
            .ok_or_else(|| anyhow!("missing Impressum character position"))?;
        ensure!(!text.trim().is_empty(), "empty Impressum field");
        out.insert(key.into(), text);
    }
    Ok(out)
}

pub(super) fn default_out() -> Result<PathBuf> {
    let home = std::env::var_os("HOME").ok_or_else(|| anyhow!("home directory unavailable"))?;
    Ok(PathBuf::from(home).join(".local/share/baylee/impressum.bin"))
}

// Resolve existing symlinks *before* processing '..'; a lexical cleanup first
// would let 'outside/link/../file' evade the real filesystem's repository fence.
pub(super) fn resolve(path: &Path) -> Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut resolved = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::ParentDir => {
                resolved.pop();
            }
            Component::CurDir => {}
            other => {
                resolved.push(other.as_os_str());
                match fs::symlink_metadata(&resolved) {
                    Ok(_) => {
                        resolved = fs::canonicalize(&resolved)?;
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                    Err(e) => return Err(e.into()),
                }
            }
        }
    }
    Ok(resolved)
}

pub(super) fn outside(root: &Path, output: &Path) -> Result<PathBuf> {
    let root = fs::canonicalize(root).map_err(|_| anyhow!("cannot resolve repository root"))?;
    let path = resolve(output).map_err(|_| anyhow!("cannot resolve Impressum output path"))?;
    ensure!(
        !path.starts_with(root) && !path.ancestors().any(|p| p.join(".git").exists()),
        "refused: Impressum output is inside the repository or another Git checkout"
    );
    Ok(path)
}

pub(super) fn write_blob(root: &Path, input: &Path, output: &Path) -> Result<usize> {
    let output = outside(root, output)?;
    let input = fs::File::open(input).map_err(|_| anyhow!("cannot read private Impressum JSON"))?;
    let mut bytes = Vec::new();
    input
        .take((MAX_BLOB + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| anyhow!("cannot read private Impressum JSON"))?;
    ensure!(bytes.len() <= MAX_BLOB, "private Impressum JSON too large");
    let fields: Value =
        serde_json::from_slice(&bytes).map_err(|_| anyhow!("invalid private Impressum JSON"))?;
    let blob = encode(&fields, None)?;
    let parent = output
        .parent()
        .ok_or_else(|| anyhow!("Impressum output has no parent"))?;
    fs::create_dir_all(parent).map_err(|_| anyhow!("cannot create Impressum output directory"))?;
    let output = outside(root, &output)?;
    let mut temp = tempfile::NamedTempFile::new_in(output.parent().unwrap())
        .map_err(|_| anyhow!("cannot create temporary Impressum output"))?;
    writeln!(temp, "{blob}").map_err(|_| anyhow!("cannot write Impressum blob"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        temp.as_file()
            .set_permissions(fs::Permissions::from_mode(0o644))
            .map_err(|_| anyhow!("cannot set Impressum output permissions"))?;
    }
    temp.persist(output)
        .map_err(|_| anyhow!("cannot replace Impressum output"))?;
    Ok(blob.len() + 1)
}
