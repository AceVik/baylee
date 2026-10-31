//! Balanced multi-table pairing scheduler, deck inventory parser, and stats summarizer.
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context as _, bail, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::audit::{sha256_file, write_atomic_json};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DeckEntry {
    pub key: String,
    pub name: String,
    pub path: String,
    pub sha256: String,
    pub format: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub source_file: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct DeckInventory {
    pub name: String,
    pub format: String,
    pub cards: Vec<String>,
    pub sideboard: Vec<String>,
    pub commanders: Vec<String>,
}

/// Parse a frozen deck file into `DeckInventory` identically to `transport.py::upload`.
pub fn parse_deck_inventory(deck: &DeckEntry) -> anyhow::Result<DeckInventory> {
    let path = Path::new(&deck.path);
    ensure!(
        path.is_file() && sha256_file(path)? == deck.sha256,
        "Deck input changed: {}",
        deck.path
    );
    let raw = std::fs::read_to_string(path)?;
    let mut cards = Vec::new();
    let mut sideboard = Vec::new();
    let mut commanders = Vec::new();
    let mut section = "cards";
    for raw_line in raw.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower == "[commander]"
            || lower == "[commanders]"
            || lower == "commander"
            || lower == "commander:"
        {
            section = "commanders";
            continue;
        }
        if lower == "[sideboard]" || lower == "sideboard" || lower == "sideboard:" {
            section = "sideboard";
            continue;
        }
        // `[deck:Name]` opens a deck file in the repository's own format
        // (`xtask deck-check`, every corpus deck); the manifest names it.
        if lower == "[deck]" || lower == "deck" || lower == "deck:" || lower.starts_with("[deck:") {
            section = "cards";
            continue;
        }
        let Some((count_str, card_name)) = line.split_once(' ') else {
            bail!("Unsupported frozen deck row in {}: {line}", deck.path);
        };
        let count: usize = count_str
            .parse()
            .with_context(|| format!("Invalid count in {}: {line}", deck.path))?;
        let card_name = card_name.trim();
        ensure!(!card_name.is_empty(), "Empty card name in {}", deck.path);
        match section {
            "commanders" => {
                for _ in 0..count {
                    commanders.push(card_name.to_owned());
                }
            }
            "sideboard" => sideboard.push(format!("{count} {card_name}")),
            _ => cards.push(format!("{count} {card_name}")),
        }
    }
    ensure!(!cards.is_empty(), "Empty deck: {}", deck.path);
    Ok(DeckInventory {
        name: deck.name.clone(),
        format: deck.format.clone(),
        cards,
        sideboard,
        commanders,
    })
}

/// Format a `DeckInventory` for `baylee-seat --deck` identically to `transport.py::cli_deck_text`.
pub fn cli_deck_text(inv: &DeckInventory) -> String {
    let mut lines =
        Vec::with_capacity(inv.cards.len() + inv.sideboard.len() + inv.commanders.len() + 6);
    lines.push("Deck".to_owned());
    lines.extend(inv.cards.iter().cloned());
    if !inv.sideboard.is_empty() {
        lines.push(String::new());
        lines.push("Sideboard".to_owned());
        lines.extend(inv.sideboard.iter().cloned());
    }
    if !inv.commanders.is_empty() {
        lines.push(String::new());
        lines.push("Commander".to_owned());
        for c in &inv.commanders {
            lines.push(format!("1 {c}"));
        }
    }
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

/// Write `seat0-deck.txt` and `seat1-deck.txt` inside `out_dir`.
pub fn derive_cli_decks(
    out_dir: &Path,
    inventories: &[DeckInventory; 2],
) -> anyhow::Result<[PathBuf; 2]> {
    std::fs::create_dir_all(out_dir)?;
    let p0 = out_dir.join("seat0-deck.txt");
    let p1 = out_dir.join("seat1-deck.txt");
    ensure!(
        !p0.exists() && !p1.exists(),
        "Refuse existing derived seat deck"
    );
    std::fs::write(&p0, cli_deck_text(&inventories[0]).as_bytes())?;
    std::fs::write(&p1, cli_deck_text(&inventories[1]).as_bytes())?;
    Ok([p0, p1])
}

pub fn load_decks(manifest_path: &Path) -> anyhow::Result<Vec<DeckEntry>> {
    let raw = std::fs::read_to_string(manifest_path)
        .with_context(|| format!("reading deck manifest {}", manifest_path.display()))?;
    let val: Value = serde_json::from_str(&raw)?;
    let arr = val
        .get("decks")
        .and_then(Value::as_array)
        .context("deck manifest missing `decks` array")?;
    let mut decks: Vec<DeckEntry> = Vec::with_capacity(arr.len());
    for (idx, item) in arr.iter().enumerate() {
        let key = item
            .get("key")
            .or_else(|| item.get("deck_id"))
            .and_then(Value::as_str)
            .map_or_else(|| format!("deck-{idx:03}"), str::to_owned);
        let name = item
            .get("name")
            .or_else(|| item.get("deck_id"))
            .and_then(Value::as_str)
            .unwrap_or(&key)
            .to_owned();
        let path = item
            .get("path")
            .and_then(Value::as_str)
            .context("deck missing path")?
            .to_owned();
        let sha256 = match item.get("sha256").and_then(Value::as_str) {
            Some(s) if !s.is_empty() => s.to_owned(),
            _ => sha256_file(Path::new(&path))?,
        };
        let declared = item
            .get("format")
            .and_then(Value::as_str)
            .context("Deck manifest must explicitly declare its format")?;
        // Corpus and importer manifests name formats in lower case.
        let format = ["Commander", "Constructed", "Highlander"]
            .into_iter()
            .find(|known| known.eq_ignore_ascii_case(declared))
            .unwrap_or(declared)
            .to_owned();
        let source = item
            .get("source")
            .or_else(|| item.get("split"))
            .and_then(Value::as_str)
            .unwrap_or("manifest")
            .to_owned();
        let source_file = item
            .get("source_file")
            .and_then(Value::as_str)
            .unwrap_or(&path)
            .to_owned();
        decks.push(DeckEntry {
            key,
            name,
            path,
            sha256,
            format,
            source,
            source_file,
        });
    }
    ensure!(!decks.is_empty(), "deck manifest contains no decks");
    Ok(decks)
}

/// Explicit collector policy; Highlander groups may contain Constructed decks.
pub fn collector_room_setup(formats: [&str; 2]) -> anyhow::Result<baylee_core::preset::RoomSetup> {
    ensure!(
        formats[0] == formats[1],
        "Collector refuses mixed declared deck formats"
    );
    let (starting_life, free_mulligans) = match formats[0] {
        "Commander" => (40, 1),
        "Constructed" | "Highlander" => (20, 0),
        other => bail!("Unsupported collector deck format: {other}"),
    };
    Ok(baylee_core::preset::RoomSetup {
        starting_life,
        free_mulligans,
        ..Default::default()
    })
}

struct PairingPlan {
    pairs: Vec<[String; 2]>,
    models: BTreeMap<String, String>,
    config_sha256: String,
    source: Option<PathBuf>,
    source_sha256: Option<String>,
}

impl PairingPlan {
    fn load(config: &Path, source: Option<PathBuf>) -> anyhow::Result<Self> {
        let config_sha256 = sha256_file(config)?;
        let paths = baylee_seat::config::Paths {
            settings: Some(config.to_path_buf()),
            named: true,
            ledger: None,
        };
        let settings = paths.load().map_err(anyhow::Error::msg)?;
        let (pairs, source_sha256) = if let Some(path) = &source {
            let digest = sha256_file(path)?;
            let pairs: Vec<[String; 2]> = serde_json::from_slice(&std::fs::read(path)?)?;
            ensure!(
                sha256_file(path)? == digest,
                "Pairings plan changed while reading"
            );
            (pairs, Some(digest))
        } else {
            (
                vec![
                    ["argon-low".into(), "argon-low".into()],
                    ["argon-low".into(), "flash-low".into()],
                    ["flash-low".into(), "argon-low".into()],
                ],
                None,
            )
        };
        ensure!(!pairs.is_empty(), "Pairings plan cannot be empty");
        let mut models = BTreeMap::new();
        let mut seen = BTreeSet::new();
        for pair in &pairs {
            ensure!(
                seen.insert(pair.clone()),
                "Duplicate profile pair in pairings plan"
            );
            for profile in pair {
                let plan = baylee_seat::config::plan(
                    None,
                    settings.as_ref(),
                    &paths,
                    Some(profile),
                    &baylee_seat::config::Overrides::default(),
                )
                .map_err(anyhow::Error::msg)?
                .context("Profile has no model")?;
                let declared = baylee_seat::declare::llm(&plan.settings);
                ensure!(
                    !declared.model.is_empty(),
                    "Profile {profile} must name an explicit model"
                );
                ensure!(
                    baylee_protocol::mind::fault(&declared).is_none(),
                    "Profile {profile} has an invalid declared model"
                );
                models.insert(profile.clone(), declared.model);
            }
        }
        ensure!(
            sha256_file(config)? == config_sha256,
            "Profiles config changed while reading"
        );
        Ok(Self {
            pairs,
            models,
            config_sha256,
            source,
            source_sha256,
        })
    }

    fn validate(&self, config: &Path) -> anyhow::Result<()> {
        ensure!(
            sha256_file(config)? == self.config_sha256,
            "Profiles config changed after startup"
        );
        if let (Some(path), Some(digest)) = (&self.source, &self.source_sha256) {
            ensure!(
                sha256_file(path)? == *digest,
                "Pairings plan changed after startup"
            );
        }
        Ok(())
    }
}

pub fn now_unix() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0.0, |d| d.as_secs_f64())
}

#[derive(Clone, Debug)]
pub struct Assignment {
    pub attempt_id: String,
    pub attempt_dir: PathBuf,
    pub slot_id: usize,
    pub format: String,
    pub decks: [DeckEntry; 2],
    pub profiles: [String; 2],
    pub models: [String; 2],
    pub request: Value,
}

#[derive(Clone, Debug)]
struct InFlight {
    format: String,
    decks: [String; 2],
    profiles: [String; 2],
}

pub struct Scheduler {
    root: PathBuf,
    gateway: String,
    profiles_config: PathBuf,
    pairing_plan: PairingPlan,
    decks: Vec<DeckEntry>,
    call_timeout_seconds: u64,
    game_timeout_seconds: u64,
    next_index: usize,
    in_flight: BTreeMap<String, InFlight>,
    known_record_shas: BTreeSet<String>,
}

fn all_attempts_dirs(root: &Path) -> Vec<PathBuf> {
    let mut dirs = vec![root.join("attempts")];
    if let Some(parent) = root.parent()
        && let Ok(entries) = std::fs::read_dir(parent)
    {
        let mut extra: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                p != root
                    && p.file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|n| n.starts_with("llm-teacher-lane"))
                    && p.join("attempts").is_dir()
            })
            .map(|p| p.join("attempts"))
            .collect();
        extra.sort();
        dirs.extend(extra);
    }
    dirs
}

/// Refuse to reuse a controller while an earlier attempt may still own a room
/// or a transport. A receipt alone does not prove cleanup.
pub fn attempt_closed(dir: &Path) -> anyhow::Result<()> {
    for marker in [
        "transport-blocked",
        "transport-blocked.json",
        "TRANSPORT_BLOCKED",
    ] {
        ensure!(
            !dir.join(marker).exists(),
            "Transport cleanup unresolved in {}",
            dir.display()
        );
    }
    let room = dir.join("room-state.json");
    if !room.exists() {
        ensure!(
            !dir.join("request.json").exists(),
            "Missing room ownership evidence in {}",
            dir.display()
        );
        return Ok(());
    }
    let state: Value = serde_json::from_slice(&std::fs::read(room)?)?;
    ensure!(
        state.get("status").and_then(Value::as_str) == Some("closed"),
        "Room ownership unresolved in {}",
        dir.display()
    );
    let proof = state.get("proof").and_then(Value::as_str).unwrap_or("");
    ensure!(
        matches!(
            proof,
            "Complete official exported end row plus both exact seat GameOver transcripts"
                | "Complete official exported End, authenticated waiting-room cancellation, or authenticated absence before any start request"
        ) || state.get("closure_verified").and_then(Value::as_bool) == Some(true),
        "Room closure lacks verified evidence in {}",
        dir.display()
    );
    Ok(())
}

impl Scheduler {
    pub fn new(
        root: PathBuf,
        gateway: String,
        profiles_config: PathBuf,
        pairings_file: Option<PathBuf>,
        deck_manifest: &Path,
        call_timeout_seconds: u64,
        game_timeout_seconds: u64,
    ) -> anyhow::Result<Self> {
        for marker in [
            "transport-blocked",
            "transport-blocked.json",
            "TRANSPORT_BLOCKED",
        ] {
            ensure!(
                !root.join(marker).exists(),
                "Controller transport cleanup unresolved"
            );
        }
        let pairing_plan = PairingPlan::load(&profiles_config, pairings_file)?;
        let decks = load_decks(deck_manifest)?;
        for deck in &decks {
            collector_room_setup([&deck.format, &deck.format])?;
        }
        let attempts_dir = root.join("attempts");
        std::fs::create_dir_all(&attempts_dir)?;
        let mut max_idx: Option<usize> = None;
        let mut known_record_shas = BTreeSet::new();
        for scan_dir in all_attempts_dirs(&root) {
            let is_primary = scan_dir == attempts_dir;
            if let Ok(entries) = std::fs::read_dir(&scan_dir) {
                let mut dirs: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
                dirs.sort();
                for dir in dirs {
                    let name = dir.file_name().unwrap_or_default().to_string_lossy();
                    let Some(num_str) = name.strip_prefix("attempt-") else {
                        continue;
                    };
                    let Ok(idx) = num_str.parse::<usize>() else {
                        continue;
                    };
                    if is_primary {
                        max_idx = Some(max_idx.map_or(idx, |m| m.max(idx)));
                    }
                    let receipt_path = dir.join("receipt.json");
                    if receipt_path.is_file() {
                        if let Ok(raw) = std::fs::read_to_string(&receipt_path)
                            && let Ok(val) = serde_json::from_str::<Value>(&raw)
                            && let Some(rec_sha) = val
                                .get("technical")
                                .and_then(|t| t.get("record_sha256"))
                                .and_then(Value::as_str)
                        {
                            known_record_shas.insert(rec_sha.to_owned());
                        }
                    } else if is_primary {
                        // Preserve unfinished evidence; recovery must establish closure first.
                        attempt_closed(&dir)?;
                    }
                    if is_primary {
                        attempt_closed(&dir)?;
                    }
                }
            }
        }
        Ok(Self {
            root,
            gateway,
            profiles_config,
            pairing_plan,
            decks,
            call_timeout_seconds,
            game_timeout_seconds,
            next_index: max_idx.map_or(0, |m| m + 1),
            in_flight: BTreeMap::new(),
            known_record_shas,
        })
    }

    pub fn load_receipts(&self) -> Vec<Value> {
        let mut out = Vec::new();
        for attempts_dir in all_attempts_dirs(&self.root) {
            let Ok(entries) = std::fs::read_dir(&attempts_dir) else {
                continue;
            };
            let mut paths: Vec<PathBuf> = entries
                .flatten()
                .map(|e| e.path().join("receipt.json"))
                .filter(|p| p.is_file())
                .collect();
            paths.sort();
            for p in paths {
                if let Ok(raw) = std::fs::read_to_string(&p)
                    && let Ok(val) = serde_json::from_str::<Value>(&raw)
                {
                    out.push(val);
                }
            }
        }
        out
    }

    /// Register a newly admitted record SHA-256 and ensure uniqueness across all attempts.
    pub fn register_record_sha(&mut self, sha: &str) -> bool {
        self.known_record_shas.insert(sha.to_owned())
    }

    /// Allocate the next balanced pairing across all formats, decks, and model seats.
    #[allow(clippy::too_many_lines)]
    pub fn next_assignment(&mut self, slot_id: usize) -> anyhow::Result<Assignment> {
        self.pairing_plan.validate(&self.profiles_config)?;
        for marker in [
            "transport-blocked",
            "transport-blocked.json",
            "TRANSPORT_BLOCKED",
        ] {
            ensure!(
                !self.root.join(marker).exists(),
                "Controller transport cleanup unresolved"
            );
        }
        for entry in std::fs::read_dir(self.root.join("attempts"))? {
            let dir = entry?.path();
            if dir.is_dir()
                && !self.in_flight.contains_key(
                    &dir.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned(),
                )
            {
                attempt_closed(&dir)?;
            }
        }

        let receipts = self.load_receipts();
        let bucket_of = |key: &str, fmt: &str| -> String {
            if key.starts_with("hl-") {
                "Highlander".to_owned()
            } else {
                fmt.to_owned()
            }
        };
        let mut by_fmt: BTreeMap<String, Vec<DeckEntry>> = BTreeMap::new();
        for d in &self.decks {
            by_fmt
                .entry(bucket_of(&d.key, &d.format))
                .or_default()
                .push(d.clone());
        }
        by_fmt.retain(|_, v| v.len() >= 2);
        ensure!(
            !by_fmt.is_empty(),
            "Two same-format decks required in deck manifest"
        );

        let mut fmt_counts: BTreeMap<String, usize> =
            by_fmt.keys().map(|k| (k.clone(), 0)).collect();
        let mut pair_counts: BTreeMap<(String, String), usize> = BTreeMap::new();
        let mut orient_counts: BTreeMap<(String, String), usize> = BTreeMap::new();
        let mut model_orient: BTreeMap<(String, String, String, String), usize> = BTreeMap::new();
        let mut deck_games: BTreeMap<String, usize> = BTreeMap::new();

        let mut record_usage = |fmt: &str, d0: &str, d1: &str, p0: &str, p1: &str| {
            if let Some(c) = fmt_counts.get_mut(fmt) {
                *c += 1;
            }
            let key = if d0 <= d1 {
                (d0.to_owned(), d1.to_owned())
            } else {
                (d1.to_owned(), d0.to_owned())
            };
            *pair_counts.entry(key).or_insert(0) += 1;
            *orient_counts
                .entry((d0.to_owned(), d1.to_owned()))
                .or_insert(0) += 1;
            *model_orient
                .entry((d0.to_owned(), d1.to_owned(), p0.to_owned(), p1.to_owned()))
                .or_insert(0) += 1;
            *deck_games.entry(d0.to_owned()).or_insert(0) += 1;
            *deck_games.entry(d1.to_owned()).or_insert(0) += 1;
        };

        for r in &receipts {
            if r.get("status").and_then(Value::as_str) != Some("admitted") {
                continue;
            }
            let Some(req) = r.get("request") else {
                continue;
            };
            let Some(ds) = req.get("decks").and_then(Value::as_array) else {
                continue;
            };
            let Some(ps) = req.get("profiles").and_then(Value::as_array) else {
                continue;
            };
            if ds.len() == 2
                && ps.len() == 2
                && let (Some(d0), Some(d1), Some(fmt), Some(p0), Some(p1)) = (
                    ds[0].get("key").and_then(Value::as_str),
                    ds[1].get("key").and_then(Value::as_str),
                    ds[0].get("format").and_then(Value::as_str),
                    ps[0].as_str(),
                    ps[1].as_str(),
                )
            {
                let b_fmt = bucket_of(d0, fmt);
                record_usage(&b_fmt, d0, d1, p0, p1);
            }
        }
        for inf in self.in_flight.values() {
            record_usage(
                &inf.format,
                &inf.decks[0],
                &inf.decks[1],
                &inf.profiles[0],
                &inf.profiles[1],
            );
        }

        let fmt_order = |f: &str| -> usize {
            match f {
                "Commander" => 0,
                "Constructed" => 1,
                "Highlander" => 2,
                _ => 3,
            }
        };
        let chosen_fmt = by_fmt
            .keys()
            .min_by_key(|f| {
                let n_decks = by_fmt.get(*f).map_or(1, Vec::len).max(1);
                (
                    fmt_counts.get(*f).copied().unwrap_or(0) * 1000 / n_decks,
                    fmt_order(f),
                    *f,
                )
            })
            .cloned()
            .context("choosing format")?;
        let pool = &by_fmt[&chosen_fmt];
        let mut pairs = Vec::new();
        for i in 0..pool.len() {
            for j in (i + 1)..pool.len() {
                pairs.push((pool[i].clone(), pool[j].clone()));
            }
        }
        let (a, b) = pairs
            .into_iter()
            .min_by_key(|(x, y)| {
                let key = if x.key <= y.key {
                    (x.key.clone(), y.key.clone())
                } else {
                    (y.key.clone(), x.key.clone())
                };
                (
                    pair_counts.get(&key).copied().unwrap_or(0),
                    deck_games.get(&x.key).copied().unwrap_or(0)
                        + deck_games.get(&y.key).copied().unwrap_or(0),
                    x.key.clone(),
                    y.key.clone(),
                )
            })
            .context("choosing deck pair")?;

        let (seat0_deck, seat1_deck) = if orient_counts
            .get(&(a.key.clone(), b.key.clone()))
            .copied()
            .unwrap_or(0)
            <= orient_counts
                .get(&(b.key.clone(), a.key.clone()))
                .copied()
                .unwrap_or(0)
        {
            (a, b)
        } else {
            (b, a)
        };

        let chosen_profiles = self
            .pairing_plan
            .pairs
            .iter()
            .min_by_key(|ps| {
                model_orient
                    .get(&(
                        seat0_deck.key.clone(),
                        seat1_deck.key.clone(),
                        ps[0].clone(),
                        ps[1].clone(),
                    ))
                    .copied()
                    .unwrap_or(0)
            })
            .context("Choosing profile pairing")?
            .clone();
        let chosen_models = [
            self.pairing_plan.models[&chosen_profiles[0]].clone(),
            self.pairing_plan.models[&chosen_profiles[1]].clone(),
        ];

        let room_setup = collector_room_setup([&seat0_deck.format, &seat1_deck.format])?;
        let index = self.next_index;
        self.next_index += 1;
        let attempt_id = format!("attempt-{index:09}");
        let attempt_dir = self.root.join("attempts").join(&attempt_id);
        let ledger_dir = self.root.join("ledgers");
        std::fs::create_dir_all(&ledger_dir)?;
        let slot_ledger = ledger_dir.join(format!("slot-{slot_id:03}-spend.json"));

        let request = json!({
            "schema_version": 1,
            "attempt_id": attempt_id,
            "index": index,
            "rotation_block": index / 8,
            "rotation_slot": index % 8,
            "slot_id": slot_id,
            "declared_deck_format": seat0_deck.format,
            "schedule_format": chosen_fmt,
            "room_setup": room_setup,
            "decks": [seat0_deck, seat1_deck],
            "profiles": chosen_profiles,
            "models": chosen_models,
            "bc_only": true,
            "ppo_eligible": false,
            "semantic_quality_gate": false,
            "seed_contract": "Actual server RNG seed must be read from complete export; schedule index is not a game seed",
            "gateway": self.gateway,
            "profiles_config": self.profiles_config.to_string_lossy(),
            "profiles_config_sha256": self.pairing_plan.config_sha256,
            "pairings_file": self.pairing_plan.source,
            "pairings_sha256": self.pairing_plan.source_sha256,
            "pairings": self.pairing_plan.pairs,
            "ledger": slot_ledger.to_string_lossy(),
            "transcriptsdir": attempt_dir.join("transcripts").to_string_lossy(),
            "call_timeout_seconds": self.call_timeout_seconds,
            "game_timeout_seconds": self.game_timeout_seconds,
        });

        self.in_flight.insert(
            attempt_id.clone(),
            InFlight {
                format: chosen_fmt.clone(),
                decks: [seat0_deck.key.clone(), seat1_deck.key.clone()],
                profiles: chosen_profiles.clone(),
            },
        );

        Ok(Assignment {
            attempt_id,
            attempt_dir,
            slot_id,
            format: chosen_fmt,
            decks: [seat0_deck, seat1_deck],
            profiles: chosen_profiles,
            models: chosen_models,
            request,
        })
    }

    pub fn complete_assignment(&mut self, attempt_id: &str) -> anyhow::Result<Value> {
        self.in_flight.remove(attempt_id);
        self.summarize()
    }

    /// Recompute `stats.json` and `stats.csv` identically to `core.py::summarize`.
    #[allow(clippy::too_many_lines, clippy::items_after_statements)]
    pub fn summarize(&self) -> anyhow::Result<Value> {
        let rows = self.load_receipts();
        let mut records = BTreeSet::new();
        for r in &rows {
            if r.get("status").and_then(Value::as_str) == Some("admitted")
                && let Some(rec) = r
                    .get("technical")
                    .and_then(|t| t.get("record_sha256"))
                    .and_then(Value::as_str)
            {
                records.insert(rec.to_owned());
            }
        }

        #[derive(Default, Clone)]
        struct GroupStats {
            attempts: u64,
            completed: u64,
            wins: u64,
            losses: u64,
            draws: u64,
            incomplete: u64,
            errors: u64,
            fallback_answers: u64,
            llm_answers: u64,
            standing_answers: u64,
            fallback_game_count: u64,
            completed_with_no_fallback: u64,
        }

        type GroupKey = (String, String, String, String, String, usize);
        let mut groups: BTreeMap<GroupKey, GroupStats> = BTreeMap::new();

        for r in &rows {
            let Some(req) = r.get("request") else {
                continue;
            };
            let Some(decks) = req.get("decks").and_then(Value::as_array) else {
                continue;
            };
            let Some(models) = req.get("models").and_then(Value::as_array) else {
                continue;
            };
            if decks.len() < 2 || models.len() < 2 {
                continue;
            }
            let status = r.get("status").and_then(Value::as_str).unwrap_or("");
            for seat in 0..2_usize {
                let other = 1 - seat;
                let d_key = decks[seat]
                    .get("key")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_owned();
                let o_key = decks[other]
                    .get("key")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_owned();
                let fmt = decks[seat]
                    .get("format")
                    .and_then(Value::as_str)
                    .unwrap_or("Constructed")
                    .to_owned();
                let m_self = models[seat].as_str().unwrap_or("unknown").to_owned();
                let m_other = models[other].as_str().unwrap_or("unknown").to_owned();
                let key = (d_key, m_self, o_key, m_other, fmt, seat);
                let g = groups.entry(key).or_default();
                g.attempts += 1;
                if status != "admitted" {
                    g.incomplete += 1;
                    if status == "error" || status == "timeout" {
                        g.errors += 1;
                    }
                    continue;
                }
                g.completed += 1;
                let tech = r.get("technical").unwrap_or(&Value::Null);
                let term = tech.get("terminal").unwrap_or(&Value::Null);
                let is_draw = term.get("draw").and_then(Value::as_bool).unwrap_or(false);
                let winner = term.get("winner_seat").and_then(Value::as_u64);
                let seat_u64 = u64::try_from(seat).unwrap_or(0);
                if is_draw {
                    g.draws += 1;
                } else if winner == Some(seat_u64) {
                    g.wins += 1;
                } else {
                    g.losses += 1;
                }
                if let Some(counts) = tech
                    .get("seats")
                    .and_then(Value::as_array)
                    .and_then(|s| s.get(seat))
                    .and_then(|s| s.get("transcript_answer_counts"))
                {
                    let h = counts.get("house").and_then(Value::as_u64).unwrap_or(0);
                    let l = counts.get("llm").and_then(Value::as_u64).unwrap_or(0);
                    let st = counts.get("standing").and_then(Value::as_u64).unwrap_or(0);
                    g.fallback_answers += h;
                    g.llm_answers += l;
                    g.standing_answers += st;
                    if h > 0 {
                        g.fallback_game_count += 1;
                    } else {
                        g.completed_with_no_fallback += 1;
                    }
                }
            }
        }

        let mut strata = Vec::with_capacity(groups.len());
        let mut by_deck_map: BTreeMap<String, GroupStats> = BTreeMap::new();
        for ((deck, model, opp_deck, opp_model, fmt, seat), g) in &groups {
            let d = by_deck_map.entry(deck.clone()).or_default();
            d.attempts += g.attempts;
            d.completed += g.completed;
            d.wins += g.wins;
            d.losses += g.losses;
            d.draws += g.draws;
            d.incomplete += g.incomplete;
            d.errors += g.errors;
            d.fallback_answers += g.fallback_answers;
            d.llm_answers += g.llm_answers;
            d.standing_answers += g.standing_answers;
            d.fallback_game_count += g.fallback_game_count;
            d.completed_with_no_fallback += g.completed_with_no_fallback;

            let win_rate = if g.completed > 0 {
                json!(
                    f64::from(u32::try_from(g.wins).unwrap_or(0))
                        / f64::from(u32::try_from(g.completed).unwrap_or(1))
                )
            } else {
                Value::Null
            };
            strata.push(json!({
                "deck": deck,
                "model": model,
                "opponent_deck": opp_deck,
                "opponent_model": opp_model,
                "format": fmt,
                "seat": seat,
                "attempts": g.attempts,
                "completed": g.completed,
                "wins": g.wins,
                "losses": g.losses,
                "draws": g.draws,
                "incomplete": g.incomplete,
                "errors": g.errors,
                "fallback_answers": g.fallback_answers,
                "llm_answers": g.llm_answers,
                "standing_answers": g.standing_answers,
                "fallback_game_count": g.fallback_game_count,
                "completed_with_no_fallback": g.completed_with_no_fallback,
                "win_rate": win_rate,
            }));
        }

        let mut by_deck = serde_json::Map::new();
        for (deck, g) in &by_deck_map {
            let win_rate = if g.completed > 0 {
                json!(
                    f64::from(u32::try_from(g.wins).unwrap_or(0))
                        / f64::from(u32::try_from(g.completed).unwrap_or(1))
                )
            } else {
                Value::Null
            };
            by_deck.insert(
                deck.clone(),
                json!({
                    "attempts": g.attempts,
                    "completed": g.completed,
                    "wins": g.wins,
                    "losses": g.losses,
                    "draws": g.draws,
                    "incomplete": g.incomplete,
                    "errors": g.errors,
                    "fallback_answers": g.fallback_answers,
                    "llm_answers": g.llm_answers,
                    "standing_answers": g.standing_answers,
                    "fallback_game_count": g.fallback_game_count,
                    "completed_with_no_fallback": g.completed_with_no_fallback,
                    "win_rate": win_rate,
                }),
            );
        }

        let value = json!({
            "schema_version": 1,
            "by_deck": by_deck,
            "attempts": rows.len(),
            "in_flight_games": self.in_flight.len(),
            "completed_games": records.len(),
            "strata": strata,
            "denominator": "wins/losses/draws use completed games only; incomplete/error counts belong to attempts. Self-play is model/deck-conditioned, not intrinsic deck strength.",
            "semantic_quality_filter": false,
        });
        write_atomic_json(&self.root.join("stats.json"), &value)?;

        let mut csv = String::from(
            "deck,model,opponent_deck,opponent_model,format,seat,attempts,completed,wins,losses,draws,incomplete,errors,fallback_answers,llm_answers,standing_answers,fallback_game_count,completed_with_no_fallback,win_rate\n",
        );
        for ((deck, model, opp_deck, opp_model, fmt, seat), g) in &groups {
            let wr = if g.completed > 0 {
                format!(
                    "{:.6}",
                    f64::from(u32::try_from(g.wins).unwrap_or(0))
                        / f64::from(u32::try_from(g.completed).unwrap_or(1))
                )
            } else {
                String::new()
            };
            let _ = writeln!(
                csv,
                "{deck},{model},{opp_deck},{opp_model},{fmt},{seat},{},{},{},{},{},{},{},{},{},{},{},{},{wr}",
                g.attempts,
                g.completed,
                g.wins,
                g.losses,
                g.draws,
                g.incomplete,
                g.errors,
                g.fallback_answers,
                g.llm_answers,
                g.standing_answers,
                g.fallback_game_count,
                g.completed_with_no_fallback,
            );
        }
        let csv_path = self.root.join("stats.csv");
        let csv_tmp = csv_path.with_extension("csv.part");
        std::fs::write(&csv_tmp, csv.as_bytes())?;
        std::fs::rename(&csv_tmp, &csv_path)?;
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::tests::TestDir;

    fn profiles(dir: &Path) -> PathBuf {
        let path = dir.join("profiles.json");
        write_atomic_json(&path, &json!({"profiles": {
            "argon-low": {"provider":"cli", "model":"agy:gemini-4-argon-low", "game_tokens":100_000},
            "flash-low": {"provider":"cli", "model":"agy:gemini-3.8-flash-low", "game_tokens":100_000},
            "terra": {"provider":"cli", "model":"codex:gpt-6.1-terra", "game_tokens":100_000},
            "unspecified": {"provider":"cli", "model":"codex", "game_tokens":100_000}
        }})).unwrap();
        path
    }

    #[test]
    fn collector_setup_is_explicit_and_refuses_unknown_or_mixed_formats() {
        for format in ["Constructed", "Highlander"] {
            let setup = collector_room_setup([format, format]).unwrap();
            assert_eq!((setup.starting_life, setup.free_mulligans), (20, 0));
            assert!(setup.seats.is_empty());
        }
        let commander = collector_room_setup(["Commander", "Commander"]).unwrap();
        assert_eq!((commander.starting_life, commander.free_mulligans), (40, 1));
        assert!(collector_room_setup(["Commander", "Constructed"]).is_err());
        assert!(collector_room_setup(["Freeform", "Freeform"]).is_err());
        assert!(collector_room_setup(["", ""]).is_err());
    }

    #[test]
    fn pairing_plan_resolves_flash_selfplay_and_terra_from_actual_profiles() {
        let dir = TestDir::new("pairing-models");
        let config = profiles(dir.path());
        let path = dir.path().join("pairings.json");
        write_atomic_json(&path, &json!([["flash-low", "flash-low"]])).unwrap();
        let plan = PairingPlan::load(&config, Some(path.clone())).unwrap();
        assert_eq!(
            plan.pairs,
            vec![["flash-low".to_owned(), "flash-low".to_owned()]]
        );
        assert_eq!(plan.models["flash-low"], "gemini-3.8-flash-low");
        assert!(!plan.models.contains_key("argon-low"));
        write_atomic_json(
            &path,
            &json!([["terra", "flash-low"], ["flash-low", "terra"]]),
        )
        .unwrap();
        let plan = PairingPlan::load(&config, Some(path)).unwrap();
        assert_eq!(plan.models["terra"], "gpt-6.1-terra");
        assert_eq!(plan.pairs.len(), 2);
        let legacy = PairingPlan::load(&config, None).unwrap();
        assert_eq!(legacy.pairs.len(), 3);
        assert_eq!(legacy.models["argon-low"], "gemini-4-argon-low");
    }

    #[test]
    fn pairing_plan_refuses_unknown_profiles_default_models_and_changed_inputs() {
        let dir = TestDir::new("pairing-refusals");
        let config = profiles(dir.path());
        let path = dir.path().join("pairings.json");
        for pairs in [
            json!([]),
            json!([["missing", "flash-low"]]),
            json!([["unspecified", "flash-low"]]),
            json!([["flash-low"]]),
            json!([["flash-low", "flash-low"], ["flash-low", "flash-low"]]),
        ] {
            write_atomic_json(&path, &pairs).unwrap();
            assert!(PairingPlan::load(&config, Some(path.clone())).is_err());
        }
        write_atomic_json(&path, &json!([["flash-low", "flash-low"]])).unwrap();
        let plan = PairingPlan::load(&config, Some(path.clone())).unwrap();
        assert!(plan.validate(&config).is_ok());
        std::fs::write(&path, b"[]").unwrap();
        assert!(plan.validate(&config).is_err());
        let plan = PairingPlan::load(&config, None).unwrap();
        std::fs::write(&config, b"{}").unwrap();
        assert!(plan.validate(&config).is_err());
    }

    #[test]
    fn unresolved_attempts_preserve_evidence_and_block_reuse() {
        let dir = TestDir::new("unknown-room");
        write_atomic_json(
            &dir.path().join("request.json"),
            &json!({"attempt_id":"attempt-0"}),
        )
        .unwrap();
        assert!(attempt_closed(dir.path()).is_err());
        write_atomic_json(
            &dir.path().join("room-state.json"),
            &json!({"status":"unknown"}),
        )
        .unwrap();
        assert!(attempt_closed(dir.path()).is_err());
        write_atomic_json(
            &dir.path().join("room-state.json"),
            &json!({"status":"closed", "proof":"Cleaned up after error/timeout"}),
        )
        .unwrap();
        assert!(attempt_closed(dir.path()).is_err());
        write_atomic_json(
            &dir.path().join("room-state.json"),
            &json!({"status":"closed", "closure_verified":true}),
        )
        .unwrap();
        assert!(attempt_closed(dir.path()).is_ok());
        std::fs::write(dir.path().join("transport-blocked.json"), b"{}").unwrap();
        assert!(attempt_closed(dir.path()).is_err());
        assert!(!dir.path().join("receipt.json").exists());
    }

    #[test]
    fn parse_and_derive_commander_and_constructed_decks() {
        let dir = TestDir::new("sched1");
        let cmd_file = dir.path().join("cmd.txt");
        std::fs::write(
            &cmd_file,
            "# format: Commander\n[Commander]\n1 Atraxa, Praetors' Voice\n\n[Deck]\n99 Plains\n",
        )
        .unwrap();
        let con_file = dir.path().join("con.txt");
        std::fs::write(
            &con_file,
            "# adapted from a precon\n[deck:Burn]\n60 Mountain\n\n[Sideboard]\n15 Abrade\n",
        )
        .unwrap();
        let cmd_entry = DeckEntry {
            key: "cmd-1".to_owned(),
            name: "Atraxa".to_owned(),
            path: cmd_file.to_string_lossy().into_owned(),
            sha256: sha256_file(&cmd_file).unwrap(),
            format: "Commander".to_owned(),
            source: "test".to_owned(),
            source_file: String::new(),
        };
        let con_entry = DeckEntry {
            key: "con-1".to_owned(),
            name: "Burn".to_owned(),
            path: con_file.to_string_lossy().into_owned(),
            sha256: sha256_file(&con_file).unwrap(),
            format: "Constructed".to_owned(),
            source: "test".to_owned(),
            source_file: String::new(),
        };
        let inv0 = parse_deck_inventory(&cmd_entry).unwrap();
        assert_eq!(inv0.commanders, vec!["Atraxa, Praetors' Voice"]);
        assert_eq!(inv0.cards, vec!["99 Plains"]);
        let inv1 = parse_deck_inventory(&con_entry).unwrap();
        assert_eq!(inv1.cards, vec!["60 Mountain"]);
        assert_eq!(inv1.sideboard, vec!["15 Abrade"]);

        let out = dir.path().join("attempt-000000000");
        let [p0, p1] = derive_cli_decks(&out, &[inv0, inv1]).unwrap();
        let text0 = std::fs::read_to_string(p0).unwrap();
        assert!(text0.contains("Commander\n1 Atraxa, Praetors' Voice"));
        let text1 = std::fs::read_to_string(p1).unwrap();
        assert!(text1.contains("Sideboard\n15 Abrade"));
    }

    #[test]
    fn scheduler_balances_concurrent_slots_across_formats_and_decks() {
        let dir = TestDir::new("sched2");
        let mut deck_entries = Vec::new();
        for (i, fmt) in [
            (0, "Commander"),
            (1, "Commander"),
            (2, "Constructed"),
            // Importer manifests write formats in lower case.
            (3, "constructed"),
            (4, "Highlander"),
            (5, "highlander"),
        ] {
            let p = dir.path().join(format!("d{i}.txt"));
            std::fs::write(&p, "60 Plains\n").unwrap();
            deck_entries.push(json!({
                "key": format!("d-{i}"),
                "name": format!("Deck {i}"),
                "path": p,
                "sha256": sha256_file(&p).unwrap(),
                "format": fmt,
            }));
        }
        let manifest = dir.path().join("deck-manifest.json");
        write_atomic_json(&manifest, &json!({ "decks": deck_entries })).unwrap();
        write_atomic_json(&dir.path().join("profiles.json"), &json!({"profiles": {
            "argon-low": {"provider":"cli", "model":"agy:gemini-4-argon-low", "game_tokens":100_000},
            "flash-low": {"provider":"cli", "model":"agy:gemini-3.8-flash-low", "game_tokens":100_000}
        }})).unwrap();
        let mut sched = Scheduler::new(
            dir.path().to_path_buf(),
            "http://127.0.0.1:28768".to_owned(),
            dir.path().join("profiles.json"),
            None,
            &manifest,
            90,
            3600,
        )
        .unwrap();

        let a0 = sched.next_assignment(0).unwrap();
        let a1 = sched.next_assignment(1).unwrap();
        let a2 = sched.next_assignment(2).unwrap();
        assert_eq!(a0.request["room_setup"]["starting_life"], 40);
        assert_eq!(a0.request["room_setup"]["free_mulligans"], 1);
        assert_eq!(a1.request["room_setup"]["starting_life"], 20);
        assert_eq!(a1.request["room_setup"]["free_mulligans"], 0);
        assert_eq!(a2.request["room_setup"]["starting_life"], 20);
        assert_eq!(a0.format, "Commander");
        assert_eq!(a1.format, "Constructed");
        assert_eq!(a2.format, "Highlander");
    }
}
