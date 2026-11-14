//! Deduplicated collection, enrichment, and optional Argon verification of
//! rules/engine reports (`reports.jsonl`) filed by language-model seats across
//! `llm_collector4` runs.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use anyhow::{Context as _, Result};
use baylee_core::ids::CardIndex;
use baylee_seat::lookup::Library;
use baylee_train::housedeck::HouseDeck;
use baylee_train::working::repo_root;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::audit::read_jsonl;

/// Complete report summary and deduplicated audit entries across collector roots.
#[derive(Debug, Serialize)]
pub struct AuditSummary {
    pub roots: Vec<String>,
    pub total_attempts: usize,
    pub admitted_games: usize,
    pub distinct_decks_seated: usize,
    pub distinct_cards_in_seated_decks: usize,
    pub distinct_nonbasic_cards_in_seated_decks: usize,
    pub raw_reports_count: usize,
    pub deduplicated_reports_count: usize,
    pub reports_by_model: BTreeMap<String, usize>,
    pub reports_by_kind: BTreeMap<String, usize>,
    pub model_bug_benchmark: Vec<ModelBugBenchmark>,
    pub entries: Vec<AuditEntry>,
}

/// Per-profile / per-model bug-reporting volume, precision (true-positive rate),
/// and true-bug discovery yield normalized by games and LLM decisions.
#[derive(Debug, Clone, Serialize)]
pub struct ModelBugBenchmark {
    pub profile: String,
    pub model: String,
    pub games: usize,
    pub mind_decisions: u64,
    pub raw_reports: usize,
    pub unique_clusters: usize,
    pub true_bugs_raw: usize,
    pub true_bugs_unique: usize,
    pub new_bugs_unique: usize,
    pub known_bugs_unique: usize,
    pub false_positives_raw: usize,
    pub false_positives_unique: usize,
    pub unverified_raw: usize,
    pub precision_pct: Option<f64>,
    pub reports_per_10_games: f64,
    pub true_bugs_per_10_games: f64,
    pub true_bugs_per_1k_decisions: f64,
}

/// Code-verified ground-truth verdict overriding or confirming automated review.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualVerdict {
    /// `"confirmed_bug"`, `"known_bug"`, or `"false_positive"`.
    pub verdict: String,
    pub category: Option<String>,
    pub root_cause: Option<String>,
}

/// One deduplicated rules/engine report cluster with Baylee card & transcript context.
#[derive(Debug, Serialize)]
pub struct AuditEntry {
    pub cluster_id: String,
    pub dedupe_keys: Vec<String>,
    pub kind: String,
    pub highest_severity: String,
    pub highest_confidence: String,
    pub occurrences: usize,
    pub reporting_models: Vec<String>,
    pub reporting_profiles: Vec<String>,
    pub games: Vec<String>,
    pub cards: Vec<CardContext>,
    pub rules: Vec<String>,
    pub oracle_quote_matches: Option<bool>,
    pub rules_format_ok: Option<bool>,
    pub observed: String,
    pub expected: String,
    pub decision: Value,
    pub transcript_context: Option<TranscriptContext>,
    pub argon_review: Option<ArgonReview>,
    pub manual_verdict: Option<ManualVerdict>,
    pub effective_verdict: String,
}

/// Baylee's compiled card text and Rust DSL source snippet for a reported card.
#[derive(Debug, Clone, Serialize)]
pub struct CardContext {
    pub name: String,
    pub oracle_quote: Option<String>,
    pub baylee_card_text: String,
    pub card_rs_path: Option<String>,
    pub card_rs_snippet: Option<String>,
}

/// Exact prompt/answer context from the reporting seat's transcript.
#[derive(Debug, Clone, Serialize)]
pub struct TranscriptContext {
    pub attempt_dir: String,
    pub seat: u64,
    pub turn: Option<u64>,
    pub question: u64,
    pub asked_excerpt: Option<String>,
    pub plan: Option<String>,
    pub pick: Value,
}

/// Structured audit verdict produced by `gemini-4-argon-high`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArgonReview {
    pub verdict: String,
    pub confidence: String,
    pub title_de: String,
    pub cr_analysis: String,
    pub code_analysis: String,
    pub summary_de: String,
}

#[derive(Default)]
struct ProfileSeatTotals {
    model: String,
    games: usize,
    mind_decisions: u64,
}

#[derive(Clone)]
struct RawHit {
    attempt_dir: PathBuf,
    admitted: bool,
    row: Value,
}

/// Collects, deduplicates, optionally audits with Argon, applies manual verdicts,
/// computes per-model bug-hunter precision/yield stats, and formats the report.
pub fn run(
    roots: &[PathBuf],
    as_json: bool,
    argon_audit: bool,
    verdicts_path: Option<&Path>,
) -> Result<String> {
    let card_files = index_card_rs_files(&repo_root().join("crates/cards/sets"));
    let manual_map = load_manual_verdicts(verdicts_path)?;
    let (mut summary, seat_totals, raw_hits) = gather_summary(roots, &card_files)?;
    if argon_audit && !summary.entries.is_empty() {
        run_argon_reviews(&mut summary.entries)?;
    }
    apply_effective_verdicts(&mut summary.entries, &manual_map);
    summary.model_bug_benchmark =
        compute_bug_hunter_benchmark(&seat_totals, &raw_hits, &summary.entries);
    if as_json {
        Ok(format!("{}\n", serde_json::to_string_pretty(&summary)?))
    } else {
        Ok(render_markdown(&summary))
    }
}

fn load_manual_verdicts(path: Option<&Path>) -> Result<BTreeMap<String, ManualVerdict>> {
    let Some(path) = path else {
        return Ok(BTreeMap::new());
    };
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("reading verdicts file {}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("parsing verdicts file {}", path.display()))
}

fn gather_summary(
    roots: &[PathBuf],
    card_files: &BTreeMap<String, PathBuf>,
) -> Result<(
    AuditSummary,
    BTreeMap<String, ProfileSeatTotals>,
    Vec<RawHit>,
)> {
    let mut total_attempts = 0_usize;
    let mut admitted_games = 0_usize;
    let mut deck_paths = BTreeSet::new();
    let mut raw_hits = Vec::new();
    let mut seat_totals: BTreeMap<String, ProfileSeatTotals> = BTreeMap::new();
    let mut reports_by_model: BTreeMap<String, usize> = BTreeMap::new();
    let mut reports_by_kind: BTreeMap<String, usize> = BTreeMap::new();

    for root in roots {
        let attempts_dir = root.join("attempts");
        anyhow::ensure!(
            attempts_dir.is_dir(),
            "{} holds no attempts",
            root.display()
        );
        let mut dirs: Vec<PathBuf> = std::fs::read_dir(&attempts_dir)?
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();
        dirs.sort();
        for dir in dirs {
            total_attempts += 1;
            let is_admitted = std::fs::read(dir.join("receipt.json"))
                .ok()
                .and_then(|raw| serde_json::from_slice::<Value>(&raw).ok())
                .is_some_and(|receipt| {
                    if receipt["status"] == "admitted" {
                        collect_seated_deck_paths(&receipt["request"], &mut deck_paths);
                        record_admitted_seat_totals(&receipt, &mut seat_totals);
                        true
                    } else {
                        false
                    }
                });
            if is_admitted {
                admitted_games += 1;
            }
            let reports_path = dir.join("transcripts").join(baylee_seat::reports::FILE);
            if let Ok(rows) = read_jsonl(&reports_path) {
                for row in rows {
                    let model = row["model"].as_str().unwrap_or("unknown").to_owned();
                    let kind = row["kind"].as_str().unwrap_or("unknown").to_owned();
                    *reports_by_model.entry(model).or_default() += 1;
                    *reports_by_kind.entry(kind).or_default() += 1;
                    raw_hits.push(RawHit {
                        attempt_dir: dir.clone(),
                        admitted: is_admitted,
                        row,
                    });
                }
            }
        }
    }

    let (all_cards, nonbasic_cards) = count_distinct_deck_cards(&deck_paths);
    let raw_reports_count = raw_hits.len();
    let entries = deduplicate_and_enrich(raw_hits.clone(), card_files);
    Ok((
        AuditSummary {
            roots: roots.iter().map(|p| p.display().to_string()).collect(),
            total_attempts,
            admitted_games,
            distinct_decks_seated: deck_paths.len(),
            distinct_cards_in_seated_decks: all_cards,
            distinct_nonbasic_cards_in_seated_decks: nonbasic_cards,
            raw_reports_count,
            deduplicated_reports_count: entries.len(),
            reports_by_model,
            reports_by_kind,
            model_bug_benchmark: Vec::new(),
            entries,
        },
        seat_totals,
        raw_hits,
    ))
}

fn record_admitted_seat_totals(
    receipt: &Value,
    seat_totals: &mut BTreeMap<String, ProfileSeatTotals>,
) {
    let req = &receipt["request"];
    let tech = &receipt["technical"];
    for seat in 0..2 {
        let profile = req["profiles"][seat]
            .as_str()
            .or_else(|| req["models"][seat].as_str())
            .unwrap_or("unknown");
        if profile.starts_with("house:") {
            continue;
        }
        let model = req["models"][seat]
            .as_str()
            .or_else(|| tech["seats"][seat]["model"].as_str())
            .unwrap_or(profile);
        let llm_ans = tech["seats"][seat]["transcript_answer_counts"]["llm"]
            .as_u64()
            .unwrap_or(0);
        let entry = seat_totals.entry(profile.to_owned()).or_default();
        if entry.model.is_empty() {
            model.clone_into(&mut entry.model);
        }
        entry.games += 1;
        entry.mind_decisions += llm_ans;
    }
}

fn collect_seated_deck_paths(request: &Value, out: &mut BTreeSet<String>) {
    if let Some(decks) = request["decks"].as_array() {
        for deck in decks {
            if let Some(path) = deck["path"].as_str() {
                out.insert(path.to_owned());
            }
        }
    }
}

fn count_distinct_deck_cards(deck_paths: &BTreeSet<String>) -> (usize, usize) {
    use baylee_core::generated::index::{
        FOREST, ISLAND, MOUNTAIN, PLAINS, SNOW_COVERED_FOREST, SNOW_COVERED_ISLAND,
        SNOW_COVERED_MOUNTAIN, SNOW_COVERED_PLAINS, SNOW_COVERED_SWAMP, SNOW_COVERED_WASTES, SWAMP,
        WASTES,
    };
    let basics = [
        PLAINS,
        ISLAND,
        SWAMP,
        MOUNTAIN,
        FOREST,
        WASTES,
        SNOW_COVERED_PLAINS,
        SNOW_COVERED_ISLAND,
        SNOW_COVERED_SWAMP,
        SNOW_COVERED_MOUNTAIN,
        SNOW_COVERED_FOREST,
        SNOW_COVERED_WASTES,
    ];
    let mut all: BTreeSet<CardIndex> = BTreeSet::new();
    let mut nonbasic: BTreeSet<CardIndex> = BTreeSet::new();
    for path in deck_paths {
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        let Ok(parsed) = HouseDeck::parse_any(path, &text) else {
            continue;
        };
        for entry in parsed.deck.main.iter().chain(&parsed.deck.commanders) {
            all.insert(entry.index);
            if !basics.contains(&entry.index) {
                nonbasic.insert(entry.index);
            }
        }
    }
    (all.len(), nonbasic.len())
}

fn cluster_key_for(row: &Value) -> String {
    let kind = row["kind"].as_str().unwrap_or("unknown");
    let mut names: Vec<String> = row["cards"].as_array().map_or_else(Vec::new, |arr| {
        arr.iter()
            .filter_map(|c| c["name"].as_str())
            .map(|s| s.trim().to_ascii_lowercase())
            .filter(|s| !s.is_empty())
            .collect()
    });
    names.sort();
    names.dedup();
    if names.is_empty() {
        let mut rules: Vec<&str> = row["rules"].as_array().map_or_else(Vec::new, |arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .collect()
        });
        rules.sort_unstable();
        rules.dedup();
        if rules.contains(&"103.5c") {
            return format!("{kind}::cr_103.5c");
        }
        if !rules.is_empty() {
            return format!("{kind}::cr_{}", rules.join("+"));
        }
        row["dedupe_key"].as_str().unwrap_or("unkeyed").to_owned()
    } else {
        format!("{kind}::{}", names.join("+"))
    }
}

fn severity_rank(s: &str) -> u8 {
    match s {
        "game_changing" | "game_breaking" => 4,
        "major" => 3,
        "minor" => 2,
        "cosmetic" => 1,
        _ => 0,
    }
}

fn confidence_rank(s: &str) -> u8 {
    match s {
        "certain" | "high" => 3,
        "likely" | "medium" => 2,
        "unsure" | "low" => 1,
        _ => 0,
    }
}

fn hit_quality(row: &Value) -> (u8, u8, u8, usize) {
    let quote_ok = u8::from(row["oracle_quote_matches"].as_bool() == Some(true));
    let conf = confidence_rank(row["confidence"].as_str().unwrap_or(""));
    let sev = severity_rank(row["severity"].as_str().unwrap_or(""));
    let len =
        row["observed"].as_str().map_or(0, str::len) + row["expected"].as_str().map_or(0, str::len);
    (quote_ok, conf, sev, len)
}

fn deduplicate_and_enrich(
    hits: Vec<RawHit>,
    card_files: &BTreeMap<String, PathBuf>,
) -> Vec<AuditEntry> {
    let mut grouped: BTreeMap<String, Vec<RawHit>> = BTreeMap::new();
    for hit in hits {
        grouped
            .entry(cluster_key_for(&hit.row))
            .or_default()
            .push(hit);
    }
    let repo = repo_root();
    let mut entries: Vec<AuditEntry> = grouped
        .into_iter()
        .map(|(cluster_id, group)| build_entry(cluster_id, &group, card_files, &repo))
        .collect();
    entries.sort_by(|a, b| {
        severity_rank(&b.highest_severity)
            .cmp(&severity_rank(&a.highest_severity))
            .then_with(|| b.occurrences.cmp(&a.occurrences))
            .then_with(|| a.cluster_id.cmp(&b.cluster_id))
    });
    entries
}

fn build_entry(
    cluster_id: String,
    group: &[RawHit],
    card_files: &BTreeMap<String, PathBuf>,
    repo: &Path,
) -> AuditEntry {
    let best = group
        .iter()
        .max_by_key(|h| hit_quality(&h.row))
        .expect("non-empty cluster");
    let row = &best.row;
    let mut dedupe_keys = BTreeSet::new();
    let mut models = BTreeSet::new();
    let mut profiles = BTreeSet::new();
    let mut games = BTreeSet::new();
    let mut rules = BTreeSet::new();
    let mut best_sev = "unknown";
    let mut best_conf = "unknown";

    for h in group {
        if let Some(k) = h.row["dedupe_key"].as_str() {
            dedupe_keys.insert(k.to_owned());
        }
        if let Some(m) = h.row["model"].as_str() {
            models.insert(m.to_owned());
        }
        if let Some(p) = h.row["profile"].as_str() {
            profiles.insert(p.to_owned());
        }
        if let Some(g) = h.row["game_id"].as_str() {
            games.insert(g.to_owned());
        }
        if let Some(arr) = h.row["rules"].as_array() {
            for r in arr.iter().filter_map(Value::as_str) {
                rules.insert(r.to_owned());
            }
        }
        let s = h.row["severity"].as_str().unwrap_or("unknown");
        if severity_rank(s) > severity_rank(best_sev) {
            best_sev = s;
        }
        let c = h.row["confidence"].as_str().unwrap_or("unknown");
        if confidence_rank(c) > confidence_rank(best_conf) {
            best_conf = c;
        }
    }

    let cards = enrich_cards(&row["cards"], card_files, repo);
    let transcript_context = extract_transcript_context(&best.attempt_dir, row);
    AuditEntry {
        cluster_id,
        dedupe_keys: dedupe_keys.into_iter().collect(),
        kind: row["kind"].as_str().unwrap_or("unknown").to_owned(),
        highest_severity: best_sev.to_owned(),
        highest_confidence: best_conf.to_owned(),
        occurrences: group.len(),
        reporting_models: models.into_iter().collect(),
        reporting_profiles: profiles.into_iter().collect(),
        games: games.into_iter().collect(),
        cards,
        rules: rules.into_iter().collect(),
        oracle_quote_matches: row["oracle_quote_matches"].as_bool(),
        rules_format_ok: row["rules_format_ok"].as_bool(),
        observed: row["observed"].as_str().unwrap_or_default().to_owned(),
        expected: row["expected"].as_str().unwrap_or_default().to_owned(),
        decision: row["decision"].clone(),
        transcript_context,
        argon_review: None,
        manual_verdict: None,
        effective_verdict: "unverified".to_owned(),
    }
}

fn enrich_cards(
    cards_val: &Value,
    card_files: &BTreeMap<String, PathBuf>,
    repo: &Path,
) -> Vec<CardContext> {
    let Some(arr) = cards_val.as_array() else {
        return Vec::new();
    };
    let lib = Library::shared();
    arr.iter()
        .filter_map(|item| {
            let name = item["name"].as_str()?.trim().to_owned();
            if name.is_empty() {
                return None;
            }
            let oracle_quote = item["oracle_quote"].as_str().map(str::to_owned);
            let found = lib.find(&name, false, Duration::from_secs(1));
            let slug = card_slug(&name);
            let (card_rs_path, card_rs_snippet) = card_files.get(&slug).map_or((None, None), |p| {
                let rel = p.strip_prefix(repo).unwrap_or(p).display().to_string();
                let snippet = std::fs::read_to_string(p)
                    .ok()
                    .map(|s| s.lines().take(120).collect::<Vec<_>>().join("\n"));
                (Some(rel), snippet)
            });
            Some(CardContext {
                name,
                oracle_quote,
                baylee_card_text: found.text,
                card_rs_path,
                card_rs_snippet,
            })
        })
        .collect()
}

fn extract_transcript_context(attempt_dir: &Path, row: &Value) -> Option<TranscriptContext> {
    let game_id = row["game_id"].as_str()?;
    let seat = row["seat"].as_u64()?;
    let question = row["question"].as_u64()?;
    let turn = row["turn"].as_u64();
    let transcripts = attempt_dir.join("transcripts");
    let mut asked_excerpt = None;
    let mut plan = None;
    let mut pick = Value::Null;

    let mind_path = transcripts.join(format!("{game_id}-seat{seat}-mind.jsonl"));
    if let Ok(mind_lines) = read_jsonl(&mind_path) {
        for entry in &mind_lines {
            if entry["question"].as_u64() != Some(question) {
                continue;
            }
            if let Some(msg) = entry["message"].as_str() {
                let marker = format!("DECISION q{question}");
                let slice = msg.find(&marker).map_or(msg, |idx| &msg[idx..]);
                asked_excerpt = Some(truncate_chars(slice, 2400));
            }
            let ans = &entry["reply"]["answer"];
            if !ans.is_null() {
                plan = ans["say"]
                    .as_str()
                    .map(str::to_owned)
                    .or_else(|| ans.get("plan").map(Value::to_string));
                pick = ans.clone();
            }
        }
    }

    let seat_path = transcripts.join(format!("{game_id}-seat{seat}.jsonl"));
    if let Ok(lines) = read_jsonl(&seat_path) {
        for entry in &lines {
            if entry["question"].as_u64() != Some(question) {
                continue;
            }
            if entry["event"].as_str() == Some("answered") {
                if plan.is_none()
                    && let Some(note_str) = entry["note"].as_str()
                    && let Ok(note) = serde_json::from_str::<Value>(note_str)
                {
                    plan = note["say"]
                        .as_str()
                        .or_else(|| note["plan"].as_str())
                        .map(str::to_owned);
                }
                if pick.is_null() {
                    pick = entry["action"].clone();
                }
            }
        }
    }

    if asked_excerpt.is_none() && pick.is_null() {
        return None;
    }
    Some(TranscriptContext {
        attempt_dir: attempt_dir.display().to_string(),
        seat,
        turn,
        question,
        asked_excerpt,
        plan,
        pick,
    })
}

fn truncate_chars(s: &str, max: usize) -> String {
    let mut out: String = s.chars().take(max).collect();
    if s.chars().count() > max {
        out.push_str(" …[truncated]");
    }
    out
}

fn card_slug(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut prev_sep = true;
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            prev_sep = false;
        } else if !prev_sep {
            out.push('_');
            prev_sep = true;
        }
    }
    while out.ends_with('_') {
        out.pop();
    }
    out
}

fn index_card_rs_files(sets_root: &Path) -> BTreeMap<String, PathBuf> {
    let mut map = BTreeMap::new();
    let mut stack = vec![sets_root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        let mut paths: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
        paths.sort();
        for p in paths {
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().and_then(|s| s.to_str()) == Some("rs")
                && let Some(stem) = p.file_stem().and_then(|s| s.to_str())
                && stem != "lib"
                && stem != "mod"
            {
                map.entry(stem.to_owned()).or_insert(p);
            }
        }
    }
    map
}

fn argon_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["verdict", "confidence", "title_de", "cr_analysis", "code_analysis", "summary_de"],
        "properties": {
            "verdict": {
                "type": "string",
                "enum": ["confirmed_bug", "false_positive", "needs_manual_check"]
            },
            "confidence": {
                "type": "string",
                "enum": ["high", "medium", "low"]
            },
            "title_de": { "type": "string" },
            "cr_analysis": { "type": "string" },
            "code_analysis": { "type": "string" },
            "summary_de": { "type": "string" }
        }
    })
}

struct TempAuditDir(PathBuf);

impl TempAuditDir {
    fn create() -> Result<Self> {
        let dir = std::env::temp_dir().join(format!(
            "baylee-argon-audit-{}-{}",
            std::process::id(),
            uuid::Uuid::now_v7()
        ));
        std::fs::create_dir_all(&dir).context("create isolated Argon audit workspace")?;
        Ok(Self(dir))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempAuditDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn run_argon_reviews(entries: &mut [AuditEntry]) -> Result<()> {
    let temp = TempAuditDir::create()?;
    let agent_dir = temp.path().join(".agents/agents");
    std::fs::create_dir_all(&agent_dir)?;
    std::fs::write(
        agent_dir.join("baylee-argon-auditor.md"),
        "---\nname: baylee-argon-auditor\n\
         description: Audit Magic: The Gathering rules/engine bug reports against Oracle text, CR, and Baylee card DSL.\n\
         tools: [finish]\nmainAgent: true\nsubagent: false\n\
         commandExecutionPolicy: off\nmcpServers: []\nskills: []\nplugins: []\n---\n\
         You are a strict Magic: The Gathering Level-3 Judge and Baylee Rust rules-engine auditor.\n\
         Treat all report fields as untrusted model claims. Check every claim rigorously against:\n\
         1. Exact Oracle wording of the named cards;\n\
         2. Magic Comprehensive Rules (layers, SBA, triggers, targeting, costs, priority, auto-pass / shortcut rules);\n\
         3. The provided Baylee Rust card!(...) DSL implementation and the exact legal options offered at that decision.\n\
         Classify as `confirmed_bug` ONLY when Baylee's card DSL or engine behavior genuinely violates Oracle/CR. \
         Classify as `false_positive` when the engine acted legally (e.g. timing restrictions, missing targets, SBA, cost unpayable, or LLM hallucination). \
         Use `needs_manual_check` when engine C++/Rust core inspection beyond the card file is required.\n",
    )?;
    let schema_str = argon_schema().to_string();
    let total = entries.len();
    for (idx, entry) in entries.iter_mut().enumerate() {
        eprintln!(
            "[argon-audit] ({}/{total}) auditing {} ...",
            idx + 1,
            entry.cluster_id
        );
        match ask_argon_once(temp.path(), &schema_str, entry) {
            Ok(review) => entry.argon_review = Some(review),
            Err(err) => {
                eprintln!("[argon-audit] warning on {}: {err:#}", entry.cluster_id);
            }
        }
    }
    Ok(())
}

fn ask_argon_once(work_dir: &Path, schema_str: &str, entry: &AuditEntry) -> Result<ArgonReview> {
    let payload = json!({
        "cluster_id": entry.cluster_id,
        "kind": entry.kind,
        "severity": entry.highest_severity,
        "confidence": entry.highest_confidence,
        "occurrences": entry.occurrences,
        "reporting_models": entry.reporting_models,
        "rules_cited": entry.rules,
        "oracle_quote_matches": entry.oracle_quote_matches,
        "observed_untrusted": entry.observed,
        "expected_untrusted": entry.expected,
        "decision_moment": entry.decision,
        "transcript_context": entry.transcript_context,
        "cards_and_baylee_dsl": entry.cards,
    });
    let prompt = format!(
        "Audit this deduplicated LLM-seat bug report and return the structured JSON verdict:\n\n{}",
        serde_json::to_string_pretty(&payload)?
    );
    let user_line = json!({
        "event": "user",
        "message": { "role": "user", "content": prompt }
    })
    .to_string();

    let mut cmd = Command::new("agy");
    cmd.args([
        "--input-format",
        "stream-json",
        "--output-format",
        "stream-json",
        "--disable-slash-commands",
        "--agent",
        "baylee-argon-auditor",
        "--model",
        "gemini-4-argon-high",
        "--effort",
        "high",
        "--json-schema",
        schema_str,
    ])
    .current_dir(work_dir)
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .env_clear();
    for key in [
        "PATH",
        "HOME",
        "USER",
        "LOGNAME",
        "LANG",
        "XDG_CONFIG_HOME",
        "XDG_DATA_HOME",
        "XDG_STATE_HOME",
    ] {
        if let Ok(val) = std::env::var(key) {
            cmd.env(key, val);
        }
    }
    let mut child = cmd.spawn().context("spawn agy for Argon audit")?;
    if let Some(mut stdin) = child.stdin.take() {
        writeln!(stdin, "{user_line}")?;
    }
    let out = child
        .wait_with_output()
        .context("wait for agy Argon audit")?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    for line in stdout.lines().rev() {
        let Ok(val) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if val["event"] == "result" {
            let res = val.get("result").unwrap_or(&val);
            if let Some(structured) = res.get("structured_output").filter(|v| v.is_object()) {
                return Ok(serde_json::from_value(structured.clone())?);
            }
            if let Some(resp) = res["response"].as_str()
                && let Ok(parsed) = serde_json::from_str::<ArgonReview>(resp)
            {
                return Ok(parsed);
            }
            anyhow::bail!("agy result lacked structured ArgonReview: {res}");
        }
    }
    anyhow::bail!(
        "agy produced no result event (exit {:?}): {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    )
}

fn apply_effective_verdicts(
    entries: &mut [AuditEntry],
    manual_map: &BTreeMap<String, ManualVerdict>,
) {
    for entry in entries {
        if let Some(mv) = manual_map.get(&entry.cluster_id) {
            entry.manual_verdict = Some(mv.clone());
            entry.effective_verdict = mv.verdict.clone();
        } else if let Some(ar) = &entry.argon_review {
            entry.effective_verdict = ar.verdict.clone();
        }
    }
}

#[derive(Default)]
struct ProfileBugAccum {
    model: String,
    games: usize,
    mind_decisions: u64,
    raw_reports: usize,
    clusters: BTreeSet<String>,
    true_bugs_raw: usize,
    new_bugs: BTreeSet<String>,
    known_bugs: BTreeSet<String>,
    false_positives_raw: usize,
    false_positives: BTreeSet<String>,
    unverified_raw: usize,
}

fn compute_bug_hunter_benchmark(
    seat_totals: &BTreeMap<String, ProfileSeatTotals>,
    raw_hits: &[RawHit],
    entries: &[AuditEntry],
) -> Vec<ModelBugBenchmark> {
    let verdict_by_cluster: BTreeMap<&str, &str> = entries
        .iter()
        .map(|e| (e.cluster_id.as_str(), e.effective_verdict.as_str()))
        .collect();
    let mut acc: BTreeMap<String, ProfileBugAccum> = BTreeMap::new();
    for (profile, totals) in seat_totals {
        acc.insert(
            profile.clone(),
            ProfileBugAccum {
                model: totals.model.clone(),
                games: totals.games,
                mind_decisions: totals.mind_decisions,
                ..ProfileBugAccum::default()
            },
        );
    }

    for hit in raw_hits {
        if !hit.admitted {
            continue;
        }
        let profile = hit.row["profile"]
            .as_str()
            .filter(|s| !s.is_empty())
            .or_else(|| hit.row["model"].as_str())
            .unwrap_or("unknown")
            .to_owned();
        let model = hit.row["model"]
            .as_str()
            .map_or_else(|| profile.clone(), str::to_owned);
        let cid = cluster_key_for(&hit.row);
        let verdict = verdict_by_cluster
            .get(cid.as_str())
            .copied()
            .unwrap_or("unverified");
        let item = acc.entry(profile).or_default();
        if item.model.is_empty() {
            item.model = model;
        }
        item.raw_reports += 1;
        item.clusters.insert(cid.clone());
        match verdict {
            "confirmed_bug" => {
                item.true_bugs_raw += 1;
                item.new_bugs.insert(cid);
            }
            "known_bug" => {
                item.true_bugs_raw += 1;
                item.known_bugs.insert(cid);
            }
            "false_positive" => {
                item.false_positives_raw += 1;
                item.false_positives.insert(cid);
            }
            _ => {
                item.unverified_raw += 1;
            }
        }
    }

    let mut rows: Vec<ModelBugBenchmark> = acc
        .into_iter()
        .map(|(profile, a)| build_model_benchmark_row(profile, a))
        .collect();
    rows.sort_by(|a, b| {
        b.true_bugs_unique
            .cmp(&a.true_bugs_unique)
            .then_with(|| b.true_bugs_raw.cmp(&a.true_bugs_raw))
            .then_with(|| {
                b.precision_pct
                    .unwrap_or(-1.0)
                    .total_cmp(&a.precision_pct.unwrap_or(-1.0))
            })
            .then_with(|| b.raw_reports.cmp(&a.raw_reports))
            .then_with(|| a.profile.cmp(&b.profile))
    });
    rows
}

fn build_model_benchmark_row(profile: String, a: ProfileBugAccum) -> ModelBugBenchmark {
    let real_u64 = |n: u64| f64::from(u32::try_from(n).unwrap_or(u32::MAX));
    let real_usz = |n: usize| f64::from(u32::try_from(n).unwrap_or(u32::MAX));
    let classified = a.true_bugs_raw + a.false_positives_raw;
    let precision_pct =
        (classified > 0).then(|| 100.0 * real_usz(a.true_bugs_raw) / real_usz(classified));
    let reports_per_10_games = if a.games == 0 {
        0.0
    } else {
        10.0 * real_usz(a.raw_reports) / real_usz(a.games)
    };
    let true_bugs_per_10_games = if a.games == 0 {
        0.0
    } else {
        10.0 * real_usz(a.true_bugs_raw) / real_usz(a.games)
    };
    let true_bugs_per_1k_decisions = if a.mind_decisions == 0 {
        0.0
    } else {
        1000.0 * real_usz(a.true_bugs_raw) / real_u64(a.mind_decisions)
    };
    let mut all_true = a.new_bugs.clone();
    all_true.extend(a.known_bugs.iter().cloned());
    ModelBugBenchmark {
        profile,
        model: a.model,
        games: a.games,
        mind_decisions: a.mind_decisions,
        raw_reports: a.raw_reports,
        unique_clusters: a.clusters.len(),
        true_bugs_raw: a.true_bugs_raw,
        true_bugs_unique: all_true.len(),
        new_bugs_unique: a.new_bugs.len(),
        known_bugs_unique: a.known_bugs.len(),
        false_positives_raw: a.false_positives_raw,
        false_positives_unique: a.false_positives.len(),
        unverified_raw: a.unverified_raw,
        precision_pct,
        reports_per_10_games,
        true_bugs_per_10_games,
        true_bugs_per_1k_decisions,
    }
}

fn render_markdown(summary: &AuditSummary) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# LLM-Seat Bug Reports Summary\n");
    let _ = writeln!(
        out,
        "- **Total attempts scanned:** {}",
        summary.total_attempts
    );
    let _ = writeln!(out, "- **Admitted games:** {}", summary.admitted_games);
    let _ = writeln!(
        out,
        "- **Distinct decks seated:** {} ({} unique cards, {} unique non-basic cards)",
        summary.distinct_decks_seated,
        summary.distinct_cards_in_seated_decks,
        summary.distinct_nonbasic_cards_in_seated_decks
    );
    let _ = writeln!(
        out,
        "- **Raw reports filed:** {} → **Deduplicated clusters:** {}\n",
        summary.raw_reports_count, summary.deduplicated_reports_count
    );
    render_benchmark_table(&mut out, &summary.model_bug_benchmark);
    for (idx, e) in summary.entries.iter().enumerate() {
        let _ = writeln!(
            out,
            "## {}. `{}` ({}, severity: `{}`, occurrences: {}, verdict: `{}`)\n",
            idx + 1,
            e.cluster_id,
            e.kind,
            e.highest_severity,
            e.occurrences,
            e.effective_verdict
        );
        let _ = writeln!(
            out,
            "- **Models:** {} (profiles: {})",
            e.reporting_models.join(", "),
            e.reporting_profiles.join(", ")
        );
        let _ = writeln!(out, "- **Rules cited:** {}", e.rules.join(", "));
        let _ = writeln!(out, "- **Observed:** {}", e.observed);
        let _ = writeln!(out, "- **Expected:** {}", e.expected);
        if let Some(mv) = &e.manual_verdict {
            let _ = writeln!(
                out,
                "- **Verified Verdict:** `{}` ({}) — {}",
                mv.verdict,
                mv.category.as_deref().unwrap_or("-"),
                mv.root_cause.as_deref().unwrap_or("-")
            );
        }
        if let Some(ar) = &e.argon_review {
            let _ = writeln!(
                out,
                "- **Argon Verdict:** `{}` (confidence `{}`) — {}\n  - **CR:** {}\n  - **Code:** {}\n",
                ar.verdict, ar.confidence, ar.summary_de, ar.cr_analysis, ar.code_analysis
            );
        } else {
            let _ = writeln!(out);
        }
    }
    out
}

fn render_benchmark_table(out: &mut String, rows: &[ModelBugBenchmark]) {
    if rows.is_empty() {
        return;
    }
    let _ = writeln!(out, "## Per-Model Bug-Hunter Benchmark\n");
    let _ = writeln!(
        out,
        "| Profile | Model | Games | Decisions | Raw Reports | Unique Clusters | True Bugs (Raw / Unique: New+Known) | False Positives | Precision | True Bugs / 10 Games | True Bugs / 1k Dec |"
    );
    let _ = writeln!(
        out,
        "| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |"
    );
    for r in rows {
        let prec = r
            .precision_pct
            .map_or_else(|| "-".to_owned(), |p| format!("{p:.0}%"));
        let _ = writeln!(
            out,
            "| `{}` | `{}` | {} | {} | {} | {} | {} / {} ({}+{}) | {} | {} | {:.2} | {:.2} |",
            r.profile,
            r.model,
            r.games,
            r.mind_decisions,
            r.raw_reports,
            r.unique_clusters,
            r.true_bugs_raw,
            r.true_bugs_unique,
            r.new_bugs_unique,
            r.known_bugs_unique,
            r.false_positives_raw,
            prec,
            r.true_bugs_per_10_games,
            r.true_bugs_per_1k_decisions,
        );
    }
    let _ = writeln!(out);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bug_hunter_benchmark_computes_precision_and_normalized_yields() {
        let mut totals = BTreeMap::new();
        totals.insert(
            "argon-high".to_owned(),
            ProfileSeatTotals {
                model: "gemini-4-argon-high".into(),
                games: 10,
                mind_decisions: 500,
            },
        );
        let hits = vec![
            RawHit {
                attempt_dir: PathBuf::new(),
                admitted: true,
                row: json!({
                    "profile": "argon-high",
                    "model": "gemini-4-argon-high",
                    "kind": "card_script",
                    "cards": [{"name": "Liar's Pendulum"}],
                }),
            },
            RawHit {
                attempt_dir: PathBuf::new(),
                admitted: true,
                row: json!({
                    "profile": "argon-high",
                    "model": "gemini-4-argon-high",
                    "kind": "rules",
                    "cards": [{"name": "Shock"}],
                }),
            },
        ];
        let mut entries = deduplicate_and_enrich(hits.clone(), &BTreeMap::new());
        let mut verdicts = BTreeMap::new();
        verdicts.insert(
            "card_script::liar's pendulum".to_owned(),
            ManualVerdict {
                verdict: "confirmed_bug".into(),
                category: Some("card_dsl".into()),
                root_cause: Some("missing guess choice".into()),
            },
        );
        verdicts.insert(
            "rules::shock".to_owned(),
            ManualVerdict {
                verdict: "false_positive".into(),
                category: None,
                root_cause: None,
            },
        );
        apply_effective_verdicts(&mut entries, &verdicts);
        let bench = compute_bug_hunter_benchmark(&totals, &hits, &entries);
        assert_eq!(bench.len(), 1);
        let b = &bench[0];
        assert_eq!(
            (b.raw_reports, b.true_bugs_raw, b.false_positives_raw),
            (2, 1, 1)
        );
        assert_eq!((b.true_bugs_unique, b.new_bugs_unique), (1, 1));
        assert!((b.precision_pct.unwrap() - 50.0).abs() < 1e-9);
        assert!((b.true_bugs_per_10_games - 1.0).abs() < 1e-9);
        assert!((b.true_bugs_per_1k_decisions - 2.0).abs() < 1e-9);
    }
}
