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

/// What a seat of one pairing entry is asked to play: for a profile, as the
/// pinned profiles file says, the mind its profile declares, then each
/// fallback that profile names (`fallback`, `docs/llm-seat.md` §"A fallback
/// profile"), in the order they take over, with the mind each declares as
/// it does; for `house:<level>` ([`house_level`]), the house at that level
/// and no fallback.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chain {
    pub mind: baylee_gamehost::record::Mind,
    pub fallbacks: Vec<(String, baylee_gamehost::record::Mind)>,
}

/// What a pairing entry starts with to seat the house rather than a
/// profile: `house:sharp` is the house at its sharp level, the opponent the
/// trained net is measured against.
const HOUSE: &str = "house:";

/// The house level a pairing entry names (`house:sharp` names `sharp`), or
/// `None` for a profile of the profiles file. Such an entry is never looked
/// up there.
pub fn house_level(entry: &str) -> Option<&str> {
    entry.strip_prefix(HOUSE)
}

/// The [`Chain`] of the house at `level`: the mind `baylee-seat join --mind
/// house --level <level>` declares (`declare::house`), and no fallback.
/// Refused for a level the house does not play (`AIProfile::NAMED`).
fn house_chain(level: &str) -> anyhow::Result<Chain> {
    let levels = baylee_core::preset::AIProfile::NAMED.map(|(name, _)| name);
    ensure!(
        levels.contains(&level),
        "No house level called {level}: the house plays {}",
        levels.join(", ")
    );
    let declared = baylee_seat::declare::house(level);
    let mind = baylee_gamehost::record::Mind::from_wire(&declared).map_err(anyhow::Error::msg)?;
    Ok(Chain {
        mind,
        fallbacks: Vec::new(),
    })
}

impl Chain {
    /// Whether the house plays the seat (a pairing's `house:<level>`).
    pub fn is_house(&self) -> bool {
        self.mind.kind == baylee_gamehost::record::MindKind::House
    }

    /// The seat's name in the request's `models`, the stats and the logs:
    /// the model its profile declares, or `house:<level>` for the house.
    pub fn model(&self) -> String {
        if self.is_house() {
            format!("{HOUSE}{}", self.mind.level)
        } else {
            self.mind.model.clone()
        }
    }

    /// The fallback profiles, in order.
    pub fn fallback_profiles(&self) -> Vec<String> {
        self.fallbacks
            .iter()
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// The minds the fallbacks declare, in order.
    pub fn fallback_minds(&self) -> Vec<baylee_gamehost::record::Mind> {
        self.fallbacks
            .iter()
            .map(|(_, mind)| mind.clone())
            .collect()
    }

    /// Every mind the seat may declare, in the order it may: the profile's,
    /// then each fallback's.
    pub fn minds(&self) -> Vec<baylee_gamehost::record::Mind> {
        std::iter::once(self.mind.clone())
            .chain(self.fallback_minds())
            .collect()
    }
}

/// `profile`'s plan in the profiles file (`paths`, read as `settings`) and
/// the mind it declares: an explicit model, in a shape the engine takes.
fn declared(
    settings: Option<&baylee_seat::config::SeatSettings>,
    paths: &baylee_seat::config::Paths,
    profile: &str,
) -> anyhow::Result<(baylee_seat::config::Plan, baylee_gamehost::record::Mind)> {
    let plan = baylee_seat::config::plan(
        None,
        settings,
        paths,
        Some(profile),
        &baylee_seat::config::Overrides::default(),
    )
    .map_err(anyhow::Error::msg)?
    .with_context(|| format!("Profile {profile} has no model"))?;
    let declared = baylee_seat::declare::llm(&plan.settings);
    ensure!(
        !declared.model.is_empty(),
        "Profile {profile} must name an explicit model"
    );
    ensure!(
        baylee_protocol::mind::fault(&declared).is_none(),
        "Profile {profile} has an invalid declared model"
    );
    let mind = baylee_gamehost::record::Mind::from_wire(&declared).map_err(anyhow::Error::msg)?;
    Ok((plan, mind))
}

/// The [`Chain`] of a pairing entry: the house's for `house:<level>`
/// ([`house_chain`]), else `profile`'s in the profiles file, refused where
/// a fallback is missing, is the profile itself or is named twice, plays an
/// agent CLI (a fallback is a model behind an API), or names no explicit
/// model, so that every fallback a request lists is one the bridge seats.
pub fn requested_chain(
    settings: Option<&baylee_seat::config::SeatSettings>,
    paths: &baylee_seat::config::Paths,
    profile: &str,
) -> anyhow::Result<Chain> {
    if let Some(level) = house_level(profile) {
        return house_chain(level);
    }
    let (plan, mind) = declared(settings, paths, profile)?;
    let mut fallbacks: Vec<(String, baylee_gamehost::record::Mind)> = Vec::new();
    for fallback in &plan.fallback {
        ensure!(
            fallback != profile,
            "Profile {profile} names itself as its fallback"
        );
        ensure!(
            fallbacks.iter().all(|(named, _)| named != fallback),
            "Profile {profile} names its fallback {fallback} twice"
        );
        let (theirs, mind) = declared(settings, paths, fallback)
            .with_context(|| format!("Fallback {fallback} of profile {profile}"))?;
        ensure!(
            theirs.settings.provider != baylee_seat::llm::Provider::Cli,
            "Fallback {fallback} of profile {profile} plays an agent CLI; a fallback is a model \
             behind an API"
        );
        fallbacks.push((fallback.clone(), mind));
    }
    Ok(Chain { mind, fallbacks })
}

struct PairingPlan {
    pairs: Vec<[String; 2]>,
    models: BTreeMap<String, String>,
    /// What a seat of each profile may declare: its mind, then its fallbacks'.
    chains: BTreeMap<String, Chain>,
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
        let mut chains = BTreeMap::new();
        let mut seen = BTreeSet::new();
        for pair in &pairs {
            ensure!(
                seen.insert(pair.clone()),
                "Duplicate profile pair in pairings plan"
            );
            // The house is the opponent a teacher is measured against;
            // against itself it measures no teacher.
            ensure!(
                !pair.iter().all(|entry| house_level(entry).is_some()),
                "Pairing {pair:?} seats the house against the house"
            );
            for profile in pair {
                let chain = requested_chain(settings.as_ref(), &paths, profile)?;
                models.insert(profile.clone(), chain.model());
                chains.insert(profile.clone(), chain);
            }
        }
        ensure!(
            sha256_file(config)? == config_sha256,
            "Profiles config changed while reading"
        );
        Ok(Self {
            pairs,
            models,
            chains,
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
    /// Whether this game's seats send their reports through the gateway:
    /// always, unless the operator set a limit ([`Scheduler::with_report_games`])
    /// and this game is past it; then they keep them in `reports.jsonl` only.
    pub send_reports: bool,
    pub request: Value,
}

impl Assignment {
    /// The seat the house plays (`house:<level>`), where it plays one: a
    /// pairing never seats it twice.
    pub fn house_seat(&self) -> Option<usize> {
        self.profiles
            .iter()
            .position(|entry| house_level(entry).is_some())
    }
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
    /// The operator's limit: attempts, counted over the whole root, whose
    /// seats send reports; `None`, every attempt's do.
    report_games: Option<usize>,
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
            report_games: None,
            in_flight: BTreeMap::new(),
            known_record_shas,
        })
    }

    /// With `Some(games)`, the operator's limit: only the first `games`
    /// attempts of this root send their seats' reports through the
    /// gateway, and every later game keeps them in `reports.jsonl`. With
    /// `None`, the default (owner, 01.11.2026), every game sends them: the
    /// gateway's budget an hour and each seat's dedupe bound how many.
    #[must_use]
    pub const fn with_report_games(mut self, games: Option<usize>) -> Self {
        self.report_games = games;
        self
    }

    /// Whether the attempt numbered `index` (over the whole root) sends its
    /// seats' reports ([`Self::with_report_games`]).
    fn sends_reports(&self, index: usize) -> bool {
        self.report_games.is_none_or(|games| index < games)
    }

    /// Whether any pairing seats the house (`house:<level>`), so that a
    /// slot needs a house account beside its teacher.
    pub fn seats_the_house(&self) -> bool {
        self.pairing_plan.chains.values().any(Chain::is_house)
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
        // Profile pairings over every game: with many decks almost every deck
        // pair is new, so balancing per deck pair alone kept the first pairing.
        let mut profile_pairs: BTreeMap<(String, String), usize> = BTreeMap::new();
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
            *profile_pairs
                .entry((p0.to_owned(), p1.to_owned()))
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
                (
                    profile_pairs
                        .get(&(ps[0].clone(), ps[1].clone()))
                        .copied()
                        .unwrap_or(0),
                    model_orient
                        .get(&(
                            seat0_deck.key.clone(),
                            seat1_deck.key.clone(),
                            ps[0].clone(),
                            ps[1].clone(),
                        ))
                        .copied()
                        .unwrap_or(0),
                )
            })
            .context("Choosing profile pairing")?
            .clone();
        let chosen_models = [
            self.pairing_plan.models[&chosen_profiles[0]].clone(),
            self.pairing_plan.models[&chosen_profiles[1]].clone(),
        ];
        // Each seat's fallbacks, in the order they take over, and the mind
        // each declares as it does: the audit holds the record to them.
        let chains = chosen_profiles
            .clone()
            .map(|p| self.pairing_plan.chains[&p].clone());

        let room_setup = collector_room_setup([&seat0_deck.format, &seat1_deck.format])?;
        let index = self.next_index;
        self.next_index += 1;
        let attempt_id = format!("attempt-{index:09}");
        let attempt_dir = self.root.join("attempts").join(&attempt_id);
        let ledger_dir = self.root.join("ledgers");
        std::fs::create_dir_all(&ledger_dir)?;
        let slot_ledger = ledger_dir.join(format!("slot-{slot_id:03}-spend.json"));
        let send_reports = self.sends_reports(index);

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
            "fallback_profiles": chains.clone().map(|c| c.fallback_profiles()),
            "requested_fallbacks": chains.map(|c| c.fallback_minds()),
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
            "send_reports": send_reports,
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
            send_reports,
            request,
        })
    }

    pub fn complete_assignment(&mut self, attempt_id: &str) -> anyhow::Result<Value> {
        self.in_flight.remove(attempt_id);
        self.summarize()
    }

    /// Recompute `stats.json` and `stats.csv` identically to `core.py::summarize`.
    ///
    /// A house seat (a pairing's `house:<level>`, its model so named) counts
    /// its answers as `house_answers`: never a model's, and never a
    /// fallback's, since it has no model to fall back from; so it counts in
    /// neither `fallback_game_count` nor `completed_with_no_fallback`. A
    /// model's seat fell back where the house answered for it or a fallback
    /// profile took over from it (`profile_fallbacks.took_over`); its
    /// `fallback_answers` are both's (the profile's, `llm_answers` counts as
    /// well).
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
            house_answers: u64,
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
                let house_seat = house_level(&m_self).is_some();
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
                let seat_facts = tech
                    .get("seats")
                    .and_then(Value::as_array)
                    .and_then(|s| s.get(seat));
                if let Some(counts) = seat_facts.and_then(|s| s.get("transcript_answer_counts")) {
                    let h = counts.get("house").and_then(Value::as_u64).unwrap_or(0);
                    let l = counts.get("llm").and_then(Value::as_u64).unwrap_or(0);
                    let st = counts.get("standing").and_then(Value::as_u64).unwrap_or(0);
                    // A fallback profile's answers (`llm_answers` counts them too).
                    let taken: u64 = seat_facts
                        .and_then(|s| s.pointer("/profile_fallbacks/played"))
                        .and_then(Value::as_array)
                        .map_or(0, |played| {
                            played
                                .iter()
                                .filter_map(|f| f.get("answered").and_then(Value::as_u64))
                                .sum()
                        });
                    let took_over = seat_facts
                        .and_then(|s| s.pointer("/profile_fallbacks/took_over"))
                        .and_then(Value::as_array)
                        .is_some_and(|t| !t.is_empty());
                    g.llm_answers += l;
                    g.standing_answers += st;
                    if house_seat {
                        g.house_answers += h;
                    } else if h > 0 || took_over {
                        g.fallback_answers += h + taken;
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
            d.house_answers += g.house_answers;
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
                "house_answers": g.house_answers,
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
                    "house_answers": g.house_answers,
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
            "deck,model,opponent_deck,opponent_model,format,seat,attempts,completed,wins,losses,draws,incomplete,errors,fallback_answers,llm_answers,standing_answers,house_answers,fallback_game_count,completed_with_no_fallback,win_rate\n",
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
                "{deck},{model},{opp_deck},{opp_model},{fmt},{seat},{},{},{},{},{},{},{},{},{},{},{},{},{},{wr}",
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
                g.house_answers,
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
        assert!(plan.chains["terra"].fallbacks.is_empty());
        let legacy = PairingPlan::load(&config, None).unwrap();
        assert_eq!(legacy.pairs.len(), 3);
        assert_eq!(legacy.models["argon-low"], "gemini-4-argon-low");
    }

    /// A profile's fallbacks are frozen with it, in the order they take
    /// over, each with the mind it declares; a chain the bridge would not
    /// seat whole (a fallback missing, the profile itself, one named twice,
    /// an agent CLI) refuses the plan.
    #[test]
    fn pairing_plan_freezes_each_profile_s_fallback_chain() {
        let dir = TestDir::new("pairing-chains");
        let config = dir.path().join("profiles.json");
        let path = dir.path().join("pairings.json");
        write_atomic_json(&path, &json!([["terra", "terra"]])).unwrap();
        let write = |fallback: Value| {
            write_atomic_json(
                &config,
                &json!({"profiles": {
                    "terra": {"provider":"cli", "model":"codex:gpt-6.1-terra",
                              "game_tokens":100_000, "fallback": fallback},
                    "api": {"provider":"openai", "model":"gpt-6.1-terra",
                            "base_url":"https://api.openai.com/v1", "game_tokens":100_000},
                    "ds": {"provider":"openai", "model":"deepseek-flash", "game_tokens":100_000},
                    "flash": {"provider":"cli", "model":"agy:gemini-3.8-flash-low",
                              "game_tokens":100_000},
                }}),
            )
            .unwrap();
        };
        write(json!(["api", "ds"]));
        let plan = PairingPlan::load(&config, Some(path.clone())).unwrap();
        let chain = &plan.chains["terra"];
        assert_eq!(chain.fallback_profiles(), ["api", "ds"]);
        assert_eq!(
            serde_json::to_value(chain.minds()).unwrap(),
            json!([
                {"kind":"llm_cli", "provider":"codex", "model":"gpt-6.1-terra"},
                {"kind":"llm_api", "provider":"openai", "model":"gpt-6.1-terra"},
                {"kind":"llm_api", "provider":"openai", "model":"deepseek-flash"},
            ])
        );
        for refused in [
            json!(["missing"]),
            json!(["terra"]),
            json!(["api", "api"]),
            json!(["flash"]),
        ] {
            write(refused.clone());
            assert!(
                PairingPlan::load(&config, Some(path.clone())).is_err(),
                "{refused}"
            );
        }
    }

    /// `house:<level>` seats the house at that level, never looked up in
    /// the profiles file: its mind is the one `join --mind house` declares,
    /// with no fallback, and its model `house:<level>`. A level the house
    /// does not play, and the house against the house, refuse the plan.
    #[test]
    fn pairing_plan_seats_the_house_at_a_level_it_plays() {
        let dir = TestDir::new("pairing-house");
        let config = profiles(dir.path());
        let path = dir.path().join("pairings.json");
        write_atomic_json(
            &path,
            &json!([
                ["house:sharp", "terra"],
                ["terra", "house:sharp"],
                ["flash-low", "house:expert"]
            ]),
        )
        .unwrap();
        let plan = PairingPlan::load(&config, Some(path.clone())).unwrap();
        assert_eq!(plan.models["house:sharp"], "house:sharp");
        assert_eq!(plan.models["house:expert"], "house:expert");
        assert_eq!(plan.models["terra"], "gpt-6.1-terra");
        let house = &plan.chains["house:sharp"];
        assert!(house.is_house() && !plan.chains["terra"].is_house());
        assert!(house.fallbacks.is_empty());
        assert_eq!(
            serde_json::to_value(house.minds()).unwrap(),
            json!([{"kind": "house", "level": "sharp"}])
        );
        for refused in [
            json!([["house:wizard", "terra"]]),
            json!([["house:", "terra"]]),
            json!([["house:Sharp", "terra"]]),
            json!([["house:sharp", "house:sharp"]]),
            json!([["terra", "terra"], ["house:steady", "house:sharp"]]),
        ] {
            write_atomic_json(&path, &refused).unwrap();
            assert!(
                PairingPlan::load(&config, Some(path.clone())).is_err(),
                "{refused}"
            );
        }
    }

    /// A scheduler of two decks and the one pairing house against `terra`.
    fn house_against_terra(dir: &TestDir) -> Scheduler {
        let decks: Vec<Value> = (0..2)
            .map(|i| {
                let p = dir.path().join(format!("d{i}.txt"));
                std::fs::write(&p, "60 Plains\n").unwrap();
                json!({"key": format!("d-{i}"), "name": format!("Deck {i}"), "path": p,
                    "sha256": sha256_file(&p).unwrap(), "format": "Constructed"})
            })
            .collect();
        let manifest = dir.path().join("deck-manifest.json");
        write_atomic_json(&manifest, &json!({ "decks": decks })).unwrap();
        let config = profiles(dir.path());
        let pairings = dir.path().join("pairings.json");
        write_atomic_json(&pairings, &json!([["house:sharp", "terra"]])).unwrap();
        Scheduler::new(
            dir.path().to_path_buf(),
            "http://127.0.0.1:28768".to_owned(),
            config,
            Some(pairings),
            &manifest,
            90,
            3600,
        )
        .unwrap()
    }

    /// A seat's answer counts as the technical result gives them.
    fn answer_counts(llm: u64, house: u64, standing: u64) -> Value {
        json!({"transcript_answer_counts":
            {"llm": llm, "house": house, "standing": standing, "other": 0}})
    }

    /// A game against the house freezes the house's model and empty chain
    /// in the request, and the stats count the house seat's answers as its
    /// own (`house_answers`), never a model's or a fallback's, while the
    /// model it played keeps its fallbacks.
    #[test]
    fn a_game_against_the_house_is_requested_and_counted_as_the_house_s() {
        let dir = TestDir::new("sched-house");
        let mut sched = house_against_terra(&dir);
        let assignment = sched.next_assignment(0).unwrap();
        let request = &assignment.request;
        assert_eq!(request["profiles"], json!(["house:sharp", "terra"]));
        assert_eq!(request["models"], json!(["house:sharp", "gpt-6.1-terra"]));
        assert_eq!(request["fallback_profiles"], json!([[], []]));
        assert_eq!(request["requested_fallbacks"], json!([[], []]));
        assert!(sched.seats_the_house());
        assert_eq!(assignment.house_seat(), Some(0));

        write_atomic_json(
            &assignment.attempt_dir.join("receipt.json"),
            &json!({
                "status": "admitted",
                "request": request,
                "technical": {
                    "record_sha256": "a".repeat(64),
                    "terminal": {"winner_seat": 1, "draw": false},
                    "seats": [answer_counts(0, 40, 3), answer_counts(30, 2, 5)],
                },
            }),
        )
        .unwrap();
        let stats = sched.complete_assignment(&assignment.attempt_id).unwrap();
        let stratum = |seat: usize| -> &Value {
            stats["strata"]
                .as_array()
                .unwrap()
                .iter()
                .find(|row| row["seat"] == seat)
                .unwrap()
        };
        let (house, model) = (stratum(0), stratum(1));
        assert_eq!(
            (&house["model"], &house["opponent_model"]),
            (&json!("house:sharp"), &json!("gpt-6.1-terra"))
        );
        assert_eq!(
            [
                &house["house_answers"],
                &house["llm_answers"],
                &house["standing_answers"]
            ],
            [&json!(40), &json!(0), &json!(3)]
        );
        assert_eq!(
            [&house["fallback_answers"], &house["fallback_game_count"]],
            [&json!(0), &json!(0)]
        );
        assert_eq!(house["completed_with_no_fallback"], 0);
        assert_eq!((&house["losses"], &model["wins"]), (&json!(1), &json!(1)));
        assert_eq!(
            [
                &model["llm_answers"],
                &model["fallback_answers"],
                &model["house_answers"]
            ],
            [&json!(30), &json!(2), &json!(0)]
        );
        assert_eq!(model["fallback_game_count"], 1);
        let csv = std::fs::read_to_string(dir.path().join("stats.csv")).unwrap();
        let header: Vec<&str> = csv.lines().next().unwrap().split(',').collect();
        let row: Vec<&str> = csv
            .lines()
            .find(|line| line.split(',').nth(1) == Some("house:sharp"))
            .unwrap()
            .split(',')
            .collect();
        assert_eq!(header.len(), row.len());
        let column = header
            .iter()
            .position(|name| *name == "house_answers")
            .unwrap();
        assert_eq!(row[column], "40");
    }

    /// A fallback profile that took over from a model is a fallback game
    /// too, its answers fallback answers as well as LLM answers.
    #[test]
    fn a_profile_that_took_over_counts_as_a_fallback() {
        let dir = TestDir::new("sched-took-over");
        let mut sched = house_against_terra(&dir);
        let second = sched.next_assignment(0).unwrap();
        let mut taken = answer_counts(25, 0, 4);
        taken["profile_fallbacks"] = json!({
            "took_over": [{"cause": "limited", "profile": "haiku", "turn": 1}],
            "played": [{"label": "haiku", "answered": 25, "turn": 1}],
        });
        write_atomic_json(
            &second.attempt_dir.join("receipt.json"),
            &json!({
                "status": "admitted",
                "request": &second.request,
                "technical": {
                    "record_sha256": "b".repeat(64),
                    "terminal": {"winner_seat": 0, "draw": false},
                    "seats": [answer_counts(0, 40, 3), taken],
                },
            }),
        )
        .unwrap();
        let stats = sched.complete_assignment(&second.attempt_id).unwrap();
        let model_total = |key: &str| -> u64 {
            stats["strata"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|row| row["model"] == "gpt-6.1-terra")
                .map(|row| row[key].as_u64().unwrap())
                .sum()
        };
        assert_eq!(
            [
                model_total("fallback_game_count"),
                model_total("fallback_answers"),
                model_total("completed_with_no_fallback"),
                model_total("llm_answers"),
            ],
            [1, 25, 0, 25]
        );
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
        // Profiles that name no fallback freeze an empty chain for each seat.
        assert_eq!(a0.request["fallback_profiles"], json!([[], []]));
        assert_eq!(a0.request["requested_fallbacks"], json!([[], []]));
        assert_eq!(a0.request["room_setup"]["free_mulligans"], 1);
        assert_eq!(a1.request["room_setup"]["starting_life"], 20);
        assert_eq!(a1.request["room_setup"]["free_mulligans"], 0);
        assert_eq!(a2.request["room_setup"]["starting_life"], 20);
        assert_eq!(a0.format, "Commander");
        assert_eq!(a1.format, "Constructed");
        assert_eq!(a2.format, "Highlander");
    }

    /// A scheduler in `dir` over 12 decks and three profiles, each paired
    /// with itself, with no limit on the games that send reports.
    fn three_efforts(dir: &TestDir) -> Scheduler {
        let decks: Vec<Value> = (0..12)
            .map(|i| {
                let p = dir.path().join(format!("d{i}.txt"));
                std::fs::write(&p, "60 Plains\n").unwrap();
                json!({"key": format!("d-{i}"), "name": format!("Deck {i}"), "path": p,
                    "sha256": sha256_file(&p).unwrap(), "format": "Constructed"})
            })
            .collect();
        let manifest = dir.path().join("deck-manifest.json");
        write_atomic_json(&manifest, &json!({ "decks": decks })).unwrap();
        let effort = |e: &str| {
            json!({"provider": "openai", "model": "deepseek-flash", "effort": e,
                "base_url": "https://api.deepseek.com/v1", "key_env": "DEEPSEEK_API_KEY",
                "game_tokens": 100_000})
        };
        write_atomic_json(
            &dir.path().join("profiles.json"),
            &json!({"profiles": {"low": effort("low"), "medium": effort("medium"), "high": effort("high")}}),
        )
        .unwrap();
        let pairings = dir.path().join("pairings.json");
        write_atomic_json(
            &pairings,
            &json!([["low", "low"], ["medium", "medium"], ["high", "high"]]),
        )
        .unwrap();
        Scheduler::new(
            dir.path().to_path_buf(),
            "http://127.0.0.1:28768".to_owned(),
            dir.path().join("profiles.json"),
            Some(pairings),
            &manifest,
            90,
            3600,
        )
        .unwrap()
    }

    /// By default every game of a root sends its seats' reports (owner,
    /// 01.11.2026): the 201st and later too, where a cap of 200 used to
    /// stop them.
    #[test]
    fn every_game_sends_its_reports_without_a_limit() {
        let dir = TestDir::new("sched-reports-all");
        let mut sched = three_efforts(&dir);
        for slot in 0..3 {
            let assignment = sched.next_assignment(slot).unwrap();
            assert!(assignment.send_reports, "game {slot}");
        }
        // The root's numbering past the old cap, as a long run reaches it.
        sched.next_index = 199;
        for slot in 0..4 {
            let assignment = sched.next_assignment(slot).unwrap();
            let index = assignment.request["index"].as_u64().unwrap();
            assert!(index >= 199);
            assert!(assignment.send_reports, "game {index}");
            assert_eq!(assignment.request["send_reports"], true, "game {index}");
        }
    }

    /// An operator's `--report-games N` sends the first N games' reports
    /// and keeps every later game's in `reports.jsonl`, counted over the
    /// whole root.
    #[test]
    fn an_operator_s_limit_stops_the_reports_after_its_games() {
        let dir = TestDir::new("sched-reports-limit");
        let mut sched = three_efforts(&dir).with_report_games(Some(250));
        sched.next_index = 248;
        let sent: Vec<(u64, bool)> = (0..4)
            .map(|slot| {
                let assignment = sched.next_assignment(slot).unwrap();
                assert_eq!(assignment.request["send_reports"], assignment.send_reports);
                (
                    assignment.request["index"].as_u64().unwrap(),
                    assignment.send_reports,
                )
            })
            .collect();
        assert_eq!(sent, [(248, true), (249, true), (250, false), (251, false)]);
    }

    #[test]
    fn profile_pairings_take_turns_even_when_every_deck_pair_is_new() {
        let dir = TestDir::new("sched-efforts");
        let mut sched = three_efforts(&dir).with_report_games(Some(2));
        let mut used: BTreeMap<String, usize> = BTreeMap::new();
        for slot in 0..6 {
            let assignment = sched.next_assignment(slot).unwrap();
            // The operator's limit: only the run's first two games send
            // their reports.
            assert_eq!(assignment.send_reports, slot < 2);
            assert_eq!(assignment.request["send_reports"], slot < 2);
            *used
                .entry(
                    assignment.request["profiles"][0]
                        .as_str()
                        .unwrap()
                        .to_owned(),
                )
                .or_default() += 1;
        }
        assert_eq!(
            used,
            BTreeMap::from([
                ("high".to_owned(), 2),
                ("low".to_owned(), 2),
                ("medium".to_owned(), 2)
            ])
        );
    }
}
