//! What a cheaper live search keeps of the full search's held-out lead,
//! replayed from the worlds of recorded search events (`eval4 --search …
//! --record-out`, `games.jsonl`; `docs/trained-ai.md` §"Lookahead search").
//!
//! A searched decision's event holds every candidate's score in every world
//! (`worlds[p][i]`), so a smaller search is the same table cut down: the first
//! `K` candidates (the policy's own answer is always the first), and the
//! answer chosen on `P` worlds and judged on the others, as `fork_advantage4`
//! judges a search (`search_labels`). A gate searches only a share of the
//! roots; the rest keep the policy's answer, a lead of 0. The **kept share**
//! is a configuration's held-out lead over the full search's on the same
//! roots, and its cost is `K × P × the share searched` (the full search is
//! `6 × 4 × 100 %`).
//!
//! The roots are the ones the search-advantage sidecar labels, found in the
//! events by the sidecar's own filter and joined to its roots in order within
//! each game and seat, checked root by root. Gates are fitted on the
//! selection half of the held-out games and read on the report half; one
//! configuration is chosen on the selection half and judged once, by the
//! pre-registration (`v6t-campaign/livesearch/`). No engine runs: only the
//! recorded worlds are read.

#![allow(clippy::missing_docs_in_private_items)]

use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead as _, BufReader};
use std::path::{Path, PathBuf};

use anyhow::{Context as _, bail, ensure};
use baylee_train::search_labels::{HALVES_MARGIN, choose, halves, paired};
use clap::Parser;
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};

/// The pre-registration this tool applies.
const PREREGISTRATION: &str = "livesearch_curve_preregistration_v1";
/// The worlds a complete root has.
const WORLDS: usize = 4;
/// Candidates, selection worlds and gate shares of the grid.
const KS: [usize; 4] = [2, 3, 4, 6];
const PS: [usize; 2] = [1, 2];
const SHARES: [f64; 5] = [0.10, 0.20, 0.30, 0.50, 1.00];
/// The rule: the share of the full lead to keep and the z to clear.
const KEEP: f64 = 0.60;
const Z: f64 = 2.0;
/// The eligible configurations' bounds.
const MAX_K: usize = 3;
const MAX_P: usize = 2;
const MAX_SHARE: f64 = 0.30;
/// The full search's cost, for scale.
const FULL_COST: f64 = 24.0;
/// Root classes: the kind of the policy's own answer (the sidecar's root
/// kinds, then the combat declarations).
const CLASSES: [&str; 7] = ["pass", "tap", "land", "activate", "cast", "attack", "block"];
/// The gates scored per root, in the gate file's order.
const SCORES: [&str; 3] = ["policy_margin", "b2_gap", "v1_gap"];

#[derive(Parser)]
#[command(about = "Replays cheaper searches from recorded worlds: a cost/gain curve")]
struct Args {
    /// A chunk: `NAME=GAMES_JSONL=SIDECAR_DIR` (its search-advantage sidecar).
    #[arg(long = "chunk", required = true)]
    chunks: Vec<String>,
    /// One chunk's gate scores, `NAME=FILE` (JSON lines: `game`, `seat`,
    /// `root_cursor`, `policy_margin`, `b2_gap`, `v1_gap`).
    #[arg(long)]
    gates: Option<String>,
    /// The pre-registration (its digest goes into the report).
    #[arg(long)]
    preregistration: PathBuf,
    /// Where the report goes, as JSON.
    #[arg(long)]
    out: PathBuf,
}

#[derive(Deserialize)]
struct Game {
    #[serde(default)]
    search_events: Vec<Event>,
    fallback_answers: Option<u64>,
    refused_answers: Option<u64>,
}

#[derive(Clone, Deserialize)]
struct Event {
    accepted: bool,
    seat: u8,
    record_input_n: u64,
    candidates: Vec<Value>,
    #[serde(default)]
    worlds: Vec<Vec<f64>>,
    chosen: usize,
    #[serde(default)]
    leaves: Leaves,
}

#[derive(Clone, Default, Deserialize)]
struct Leaves {
    #[serde(default)]
    stopped: u64,
}

/// The class of a root by the policy's own answer.
fn class_of(action: &Value) -> Option<usize> {
    let name = match action {
        Value::String(name) => name.as_str(),
        Value::Object(fields) => fields.keys().next()?.as_str(),
        _ => return None,
    };
    let at = match name {
        "PassPriority" => 0,
        "ActivateManaAbility" => 1,
        "PlayLand" => 2,
        "ActivateAbility" => 3,
        "CastSpell" => 4,
        "DeclareAttackers" => 5,
        "DeclareBlockers" => 6,
        _ => return None,
    };
    Some(at)
}

/// A root as the curve reads it.
#[derive(Clone, Debug)]
struct Root {
    chunk: usize,
    game: u32,
    seat: u8,
    /// The sidecar's root cursor; `u32::MAX` for a combat root.
    cursor: u32,
    class: usize,
    combat: bool,
    worlds: Vec<Vec<f64>>,
    /// The scored gates, where the gate file gives them.
    scores: [Option<f64>; 3],
}

/// What became of the events, by reason.
#[derive(Clone, Debug, Default, serde::Serialize)]
struct Counts {
    games: u64,
    events: u64,
    rule_roots: u64,
    combat_roots: u64,
    incomplete_worlds_rule: u64,
    incomplete_worlds_combat: u64,
    dropped: BTreeMap<String, u64>,
}

/// Events with their game id.
type Events = Vec<(u32, Event)>;

/// The events of one `games.jsonl` the sidecar's filter keeps, as (game id,
/// event), priority roots first and combat declarations apart.
fn read_events(path: &Path, counts: &mut Counts) -> anyhow::Result<(Events, Events)> {
    let file = std::fs::File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let (mut priority, mut combat) = (Vec::new(), Vec::new());
    for (line_no, line) in BufReader::new(file).lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let game: Game = serde_json::from_str(&line)
            .with_context(|| format!("{} line {}", path.display(), line_no + 1))?;
        let id = u32::try_from(line_no + 1)?;
        counts.games += 1;
        let clean = game.fallback_answers == Some(0) && game.refused_answers == Some(0);
        for event in game.search_events {
            counts.events += 1;
            let class = event.candidates.first().and_then(class_of);
            let reason = if !clean {
                Some("game_fallback")
            } else if !event.accepted {
                Some("not_accepted")
            } else if event.leaves.stopped != 0 {
                Some("stopped")
            } else if class.is_none() {
                Some("other_question")
            } else if event.worlds.is_empty()
                || event
                    .worlds
                    .iter()
                    .any(|w| w.len() != event.candidates.len())
                || event.chosen >= event.candidates.len()
            {
                Some("malformed")
            } else {
                None
            };
            match (reason, class) {
                (Some(reason), _) => *counts.dropped.entry(reason.to_owned()).or_default() += 1,
                (None, Some(c)) if c >= 5 => combat.push((id, event)),
                (None, _) => priority.push((id, event)),
            }
        }
    }
    Ok((priority, combat))
}

/// One sidecar root: (game, seat, root cursor, root kind, its searched items' advantages).
type SideRoot = (u32, u8, u32, u8, Vec<f32>);

/// The sidecar's roots, in its order.
fn read_sidecar(dir: &Path) -> anyhow::Result<Vec<SideRoot>> {
    let index =
        std::fs::read(dir.join("index.bin")).with_context(|| format!("{}", dir.display()))?;
    let cands = std::fs::read(dir.join("cands.bin"))?;
    ensure!(
        index.len() % 48 == 0 && cands.len() % 12 == 0,
        "sidecar rows have another width"
    );
    let word = |b: &[u8], at: usize| u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]]);
    let mut out = Vec::with_capacity(index.len() / 48);
    for row in index.as_chunks::<48>().0 {
        let n = word(row, 20) as usize;
        let offset = usize::try_from(u64::from_le_bytes(row[36..44].try_into()?))?;
        ensure!(
            (offset + n) * 12 <= cands.len(),
            "sidecar items out of range"
        );
        let searched = (offset..offset + n)
            .filter(|&i| cands[i * 12 + 8] == 1)
            .map(|i| f32::from_le_bytes(cands[i * 12..i * 12 + 4].try_into().expect("four bytes")))
            .collect();
        out.push((word(row, 0), row[12], word(row, 8), row[13], searched));
    }
    Ok(out)
}

/// The events' priority roots joined to the sidecar's in order within each
/// (game, seat), each checked: the same kind, as many searched items as
/// candidates, and the same paired advantages.
fn join(
    events: Vec<(u32, Event)>,
    sidecar: &[SideRoot],
    chunk: usize,
) -> anyhow::Result<Vec<Root>> {
    let mut by_seat: BTreeMap<(u32, u8), Vec<Event>> = BTreeMap::new();
    for (game, event) in events {
        by_seat.entry((game, event.seat)).or_default().push(event);
    }
    let mut side: BTreeMap<(u32, u8), Vec<&SideRoot>> = BTreeMap::new();
    for root in sidecar {
        side.entry((root.0, root.1)).or_default().push(root);
    }
    ensure!(
        by_seat.keys().collect::<BTreeSet<_>>() == side.keys().collect::<BTreeSet<_>>(),
        "chunk {chunk}: the events and the sidecar name other games or seats"
    );
    let mut out = Vec::new();
    for (key, mut events) in by_seat {
        events.sort_by_key(|e| e.record_input_n);
        let mut roots = side.remove(&key).unwrap_or_default();
        roots.sort_by_key(|r| r.2);
        ensure!(
            events.len() == roots.len(),
            "chunk {chunk} game {} seat {}: {} events, {} sidecar roots",
            key.0,
            key.1,
            events.len(),
            roots.len()
        );
        for (event, root) in events.into_iter().zip(roots) {
            let class = event
                .candidates
                .first()
                .and_then(class_of)
                .context("a joined event has no class")?;
            ensure!(
                usize::from(root.3) == class,
                "chunk {chunk} game {}: root kinds differ",
                key.0
            );
            #[allow(
                clippy::cast_possible_truncation,
                reason = "the sidecar's advantages are f32"
            )]
            let mut mine: Vec<f32> = (0..event.candidates.len())
                .map(|c| paired(&event.worlds, c).0 as f32)
                .collect();
            let mut theirs = root.4.clone();
            mine.sort_by(f32::total_cmp);
            theirs.sort_by(f32::total_cmp);
            ensure!(
                mine == theirs,
                "chunk {chunk} game {} cursor {}: the advantages differ from the sidecar's",
                key.0,
                root.2
            );
            out.push(Root {
                chunk,
                game: key.0,
                seat: key.1,
                cursor: root.2,
                class,
                combat: false,
                worlds: event.worlds,
                scores: [None; 3],
            });
        }
    }
    Ok(out)
}

/// The lead of choosing among the first `k` candidates on the worlds of
/// each half and judging on the other (`P = 2`).
fn lead_halves(worlds: &[Vec<f64>], k: usize) -> f64 {
    let cut: Vec<&[f64]> = worlds.iter().map(|w| &w[..k.min(w.len())]).collect();
    halves(&cut, HALVES_MARGIN).lead()
}

/// The lead of choosing among the first `k` candidates on one world and
/// judging on the others, averaged over which world chooses (`P = 1`).
fn lead_single(worlds: &[Vec<f64>], k: usize) -> f64 {
    let n = worlds.len();
    if n < 2 {
        return 0.0;
    }
    let mut total = 0.0;
    for w in 0..n {
        let chosen = choose(&[&worlds[w][..k.min(worlds[w].len())]], HALVES_MARGIN);
        if chosen != 0 {
            let judged: f64 = (0..n)
                .filter(|&o| o != w)
                .map(|o| worlds[o][chosen] - worlds[o][0])
                .sum();
            total += judged / real(n - 1);
        }
    }
    total / real(n)
}

/// A count as a float.
fn real(n: usize) -> f64 {
    f64::from(u32::try_from(n).unwrap_or(u32::MAX))
}

/// Which half of the held-out games a root's game is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Half {
    Selection,
    Report,
}

/// Arm B's halves: per chunk the held-out game ids (id % 4 == 0) of the rule
/// roots in ascending order, alternating selection, report.
fn halves_of(roots: &[Root]) -> BTreeMap<(usize, u32), Half> {
    let games: BTreeSet<(usize, u32)> = roots
        .iter()
        .filter(|r| !r.combat && r.game % 4 == 0)
        .map(|r| (r.chunk, r.game))
        .collect();
    let mut out = BTreeMap::new();
    let mut position: BTreeMap<usize, usize> = BTreeMap::new();
    for (chunk, game) in games {
        let at = position.entry(chunk).or_default();
        out.insert(
            (chunk, game),
            if (*at).is_multiple_of(2) {
                Half::Selection
            } else {
                Half::Report
            },
        );
        *at += 1;
    }
    out
}

/// A gate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Gate {
    None,
    Class,
    Score(usize),
}

impl Gate {
    fn name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Class => "class",
            Self::Score(at) => SCORES[at],
        }
    }
}

/// A root of an evaluation: its half, its game key, its class, its gate
/// score, its full lead and its leads per (K, P).
#[derive(Clone, Debug)]
struct Row {
    half: Half,
    game: (usize, u32),
    class: usize,
    score: Option<f64>,
    full: f64,
    leads: [[f64; 2]; 4],
}

fn rows_of(roots: &[&Root], halves: &BTreeMap<(usize, u32), Half>, gate: Gate) -> Vec<Row> {
    roots
        .iter()
        .filter_map(|root| {
            let half = *halves.get(&(root.chunk, root.game))?;
            let mut leads = [[0.0; 2]; 4];
            for (ki, &k) in KS.iter().enumerate() {
                leads[ki] = [lead_single(&root.worlds, k), lead_halves(&root.worlds, k)];
            }
            Some(Row {
                half,
                game: (root.chunk, root.game),
                class: root.class,
                score: match gate {
                    Gate::Score(at) => root.scores[at],
                    _ => None,
                },
                full: lead_halves(&root.worlds, root.worlds[0].len()),
                leads,
            })
        })
        .collect()
}

/// The roots a gate searches, fitted on the selection half for `share`: per
/// row whether it is searched.
fn fit_gate(rows: &[Row], gate: Gate, share: f64, ki: usize, pi: usize) -> (Vec<bool>, Value) {
    let selection: Vec<&Row> = rows.iter().filter(|r| r.half == Half::Selection).collect();
    let n = selection.len();
    match gate {
        Gate::None => (vec![true; rows.len()], json!({})),
        Gate::Class => {
            let mut stats: BTreeMap<usize, (usize, f64)> = BTreeMap::new();
            for row in &selection {
                let cell = stats.entry(row.class).or_default();
                cell.0 += 1;
                cell.1 += row.leads[ki][pi];
            }
            let mut ranked: Vec<(usize, usize, f64)> = stats
                .into_iter()
                .filter(|(_, (_, sum))| *sum > 0.0)
                .map(|(c, (count, sum))| (c, count, sum / real(count)))
                .collect();
            ranked.sort_by(|a, b| {
                b.2.total_cmp(&a.2)
                    .then_with(|| CLASSES[a.0].cmp(CLASSES[b.0]))
            });
            let (mut taken, mut used) = (BTreeSet::new(), 0usize);
            for (class, count, _) in ranked {
                if real(used + count) <= share * real(n) + 1e-9 {
                    taken.insert(class);
                    used += count;
                }
            }
            let names: Vec<&str> = taken.iter().map(|&c| CLASSES[c]).collect();
            (
                rows.iter().map(|r| taken.contains(&r.class)).collect(),
                json!({"classes": names}),
            )
        }
        Gate::Score(_) => {
            let mut scores: Vec<f64> = selection.iter().filter_map(|r| r.score).collect();
            scores.sort_by(|a, b| b.total_cmp(a));
            if scores.is_empty() {
                return (vec![false; rows.len()], json!({"threshold": null}));
            }
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "a count"
            )]
            let take = ((share * real(n)).ceil() as usize).clamp(1, scores.len());
            let threshold = scores[take - 1];
            (
                rows.iter()
                    .map(|r| r.score.is_some_and(|s| s >= threshold))
                    .collect(),
                json!({"threshold": threshold}),
            )
        }
    }
}

/// Per 100 roots and its standard error by game, over one half's roots given
/// as (game, value).
fn per_100(values: &[((usize, u32), f64)]) -> (f64, Option<f64>) {
    if values.is_empty() {
        return (0.0, None);
    }
    let mut games: BTreeMap<(usize, u32), f64> = BTreeMap::new();
    for (game, value) in values {
        *games.entry(*game).or_default() += value;
    }
    let r = real(values.len());
    let mean = games.values().sum::<f64>() / r * 100.0;
    let g = games.len();
    if g < 2 {
        return (mean, None);
    }
    let x: Vec<f64> = games.into_values().collect();
    let m = x.iter().sum::<f64>() / real(g);
    let var = x.iter().map(|v| (v - m).powi(2)).sum::<f64>() / real(g - 1);
    (mean, Some((real(g) * var).sqrt() / r * 100.0))
}

/// One configuration on one half.
fn half_cell(
    rows: &[Row],
    searched: &[bool],
    half: Half,
    ki: usize,
    pi: usize,
    k: usize,
    p: usize,
) -> Value {
    let picked: Vec<(&Row, bool)> = rows
        .iter()
        .zip(searched)
        .filter(|(r, _)| r.half == half)
        .map(|(r, s)| (r, *s))
        .collect();
    let cheap: Vec<_> = picked
        .iter()
        .map(|(r, on)| (r.game, if *on { r.leads[ki][pi] } else { 0.0 }))
        .collect();
    let whole: Vec<_> = picked.iter().map(|(r, _)| (r.game, r.full)).collect();
    let (lead, se) = per_100(&cheap);
    let (full, full_se) = per_100(&whole);
    let share = if picked.is_empty() {
        0.0
    } else {
        real(picked.iter().filter(|(_, s)| *s).count()) / real(picked.len())
    };
    let z = se.filter(|se| *se > 0.0).map_or(0.0, |se| lead / se);
    json!({
        "roots": picked.len(), "share": share, "lead_per_100": lead, "se": se, "z": z,
        "full_per_100": full, "full_se": full_se,
        "kept": if full > 0.0 { Some(lead / full) } else { None },
        "cost": real(k) * real(p) * share, "cost_of_full": real(k) * real(p) * share / FULL_COST,
    })
}

/// The whole grid of one population and gate.
fn grid(rows: &[Row], gate: Gate) -> Vec<Value> {
    let mut out = Vec::new();
    for (ki, &k) in KS.iter().enumerate() {
        for (pi, &p) in PS.iter().enumerate() {
            for &share in &SHARES {
                if gate == Gate::None && share < 1.0 {
                    continue;
                }
                let (searched, fitted) = fit_gate(rows, gate, share, ki, pi);
                out.push(json!({
                    "gate": gate.name(), "K": k, "P": p, "gate_share": share, "fitted": fitted,
                    "selection": half_cell(rows, &searched, Half::Selection, ki, pi, k, p),
                    "report": half_cell(rows, &searched, Half::Report, ki, pi, k, p),
                }));
            }
        }
    }
    out
}

/// The eligible configuration with the largest selection-half kept share
/// (ties: lower cost, lower K, lower P).
fn choose_configuration<'a>(cells: &'a [Value], gates: &[&str]) -> Option<&'a Value> {
    let key = |c: &Value| -> (f64, f64, u64, u64) {
        (
            c["selection"]["kept"].as_f64().unwrap_or(f64::NEG_INFINITY),
            -c["selection"]["cost"].as_f64().unwrap_or(0.0),
            u64::MAX - c["K"].as_u64().unwrap_or(0),
            u64::MAX - c["P"].as_u64().unwrap_or(0),
        )
    };
    cells
        .iter()
        .filter(|c| {
            gates.contains(&c["gate"].as_str().unwrap_or(""))
                && c["K"].as_u64().is_some_and(|k| k <= MAX_K as u64)
                && c["P"].as_u64().is_some_and(|p| p <= MAX_P as u64)
                && c["selection"]["share"]
                    .as_f64()
                    .is_some_and(|s| s <= MAX_SHARE + 1e-9)
        })
        .max_by(|a, b| {
            let (x, y) = (key(a), key(b));
            x.0.total_cmp(&y.0)
                .then(x.1.total_cmp(&y.1))
                .then(x.2.cmp(&y.2))
                .then(x.3.cmp(&y.3))
        })
}

/// The rule on a chosen configuration's report half.
fn verdict(cell: &Value) -> Value {
    let report = &cell["report"];
    let kept = report["kept"].as_f64();
    let z = report["z"].as_f64().unwrap_or(0.0);
    let checks = json!({"kept": kept.is_some_and(|k| k >= KEEP), "z": z >= Z});
    json!({
        "gate": cell["gate"], "K": cell["K"], "P": cell["P"], "gate_share": cell["gate_share"], "fitted": cell["fitted"],
        "kept": kept, "lead_per_100": report["lead_per_100"], "se": report["se"], "z": z,
        "full_per_100": report["full_per_100"], "share": report["share"], "cost": report["cost"],
        "checks": checks, "passed": checks["kept"] == true && checks["z"] == true,
    })
}

fn sha256(path: &Path) -> anyhow::Result<String> {
    use std::fmt::Write as _;
    let mut hex = String::with_capacity(64);
    for byte in Sha256::digest(std::fs::read(path)?) {
        write!(hex, "{byte:02x}")?;
    }
    Ok(hex)
}

#[allow(
    clippy::too_many_lines,
    reason = "The report's sections in the order they are written"
)]
fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let prereg: Value = serde_json::from_slice(&std::fs::read(&args.preregistration)?)?;
    ensure!(
        prereg["contract"] == PREREGISTRATION,
        "{} is not {PREREGISTRATION}",
        args.preregistration.display()
    );
    let mut names = Vec::new();
    let mut rule: Vec<Root> = Vec::new();
    let mut combat: Vec<Root> = Vec::new();
    let mut counts = Vec::new();
    for (chunk, spec) in args.chunks.iter().enumerate() {
        let parts: Vec<&str> = spec.splitn(3, '=').collect();
        let &[name, games, sidecar] = &parts[..] else {
            bail!("--chunk wants NAME=GAMES=SIDECAR")
        };
        let mut count = Counts::default();
        let (priority, declarations) = read_events(Path::new(games), &mut count)?;
        let joined = join(priority, &read_sidecar(Path::new(sidecar))?, chunk)?;
        for root in joined {
            if root.worlds.len() == WORLDS {
                count.rule_roots += 1;
                rule.push(root);
            } else {
                count.incomplete_worlds_rule += 1;
            }
        }
        for (game, event) in declarations {
            if event.worlds.len() == WORLDS {
                count.combat_roots += 1;
                let class = event
                    .candidates
                    .first()
                    .and_then(class_of)
                    .context("a combat root without class")?;
                combat.push(Root {
                    chunk,
                    game,
                    seat: event.seat,
                    cursor: u32::MAX,
                    class,
                    combat: true,
                    worlds: event.worlds,
                    scores: [None; 3],
                });
            } else {
                count.incomplete_worlds_combat += 1;
            }
        }
        names.push(name.to_owned());
        counts.push(json!({"chunk": name, "games": games, "games_sha256": sha256(Path::new(games))?, "sidecar": sidecar, "counts": count}));
    }
    // The scored gates of one chunk.
    let mut gated = None;
    if let Some(spec) = &args.gates {
        let (name, file) = spec.split_once('=').context("--gates wants NAME=FILE")?;
        let chunk = names
            .iter()
            .position(|n| n == name)
            .context("--gates names no chunk")?;
        let mut scores: BTreeMap<(u32, u8, u32), [Option<f64>; 3]> = BTreeMap::new();
        for line in BufReader::new(std::fs::File::open(file)?).lines() {
            let row: Value = serde_json::from_str(&line?)?;
            let key = (
                u32::try_from(row["game"].as_u64().context("game")?)?,
                u8::try_from(row["seat"].as_u64().context("seat")?)?,
                u32::try_from(row["root_cursor"].as_u64().context("root_cursor")?)?,
            );
            scores.insert(key, SCORES.map(|s| row[s].as_f64()));
        }
        let mut found = 0;
        for root in rule.iter_mut().filter(|r| r.chunk == chunk) {
            if let Some(s) = scores.get(&(root.game, root.seat, root.cursor)) {
                root.scores = *s;
                found += 1;
            }
        }
        gated = Some((
            chunk,
            json!({"file": file, "sha256": sha256(Path::new(file))?, "rows": scores.len(), "joined": found}),
        ));
    }
    let halves = halves_of(&rule);
    let everything: Vec<&Root> = rule.iter().collect();
    let mut populations = serde_json::Map::new();
    // The rule's population: every chunk, the gates that exist there.
    let mut pooled = Vec::new();
    for gate in [Gate::None, Gate::Class] {
        pooled.extend(grid(&rows_of(&everything, &halves, gate), gate));
    }
    let chosen = choose_configuration(&pooled, &["class", "none"]).cloned();
    let judged = chosen.as_ref().map(verdict);
    populations.insert("all_chunks".into(), json!(pooled));
    // One chunk alone with every gate, informational.
    let mut informational = serde_json::Map::new();
    if let Some((chunk, _)) = &gated {
        let mine: Vec<&Root> = rule.iter().filter(|r| r.chunk == *chunk).collect();
        let mut cells = Vec::new();
        for gate in [
            Gate::None,
            Gate::Class,
            Gate::Score(0),
            Gate::Score(1),
            Gate::Score(2),
        ] {
            cells.extend(grid(&rows_of(&mine, &halves, gate), gate));
        }
        for gate in ["class", "policy_margin", "b2_gap", "v1_gap"] {
            informational.insert(
                gate.into(),
                json!(choose_configuration(&cells, &[gate]).map(verdict)),
            );
        }
        populations.insert(names[*chunk].clone(), json!(cells));
    }
    // Combat roots added, outside the rule.
    let with_combat: Vec<&Root> = rule.iter().chain(combat.iter()).collect();
    let mut combat_cells = Vec::new();
    for gate in [Gate::None, Gate::Class] {
        combat_cells.extend(grid(&rows_of(&with_combat, &halves, gate), gate));
    }
    populations.insert("all_chunks_with_combat".into(), json!(combat_cells));
    let report = json!({
        "contract": "livesearch_curve_v1", "preregistration": args.preregistration,
        "preregistration_sha256": sha256(&args.preregistration)?, "chunks": counts, "gates": gated.map(|g| g.1),
        "halves": {"selection_games": halves.values().filter(|h| **h == Half::Selection).count(),
                   "report_games": halves.values().filter(|h| **h == Half::Report).count()},
        "chosen": chosen, "verdict": judged, "informational_single_chunk_choices": informational,
        "populations": populations,
        "converter_build": baylee_build::short(),
    });
    std::fs::write(&args.out, serde_json::to_vec_pretty(&report)?)?;
    println!(
        "{}",
        serde_json::to_string(
            &json!({"verdict": report["verdict"], "informational": report["informational_single_chunk_choices"]})
        )?
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn worlds() -> Vec<Vec<f64>> {
        // Own, pass, land: the land leads by 0.1 on every world, the pass
        // by 0.2 on world 0 only.
        vec![
            vec![0.5, 0.7, 0.6],
            vec![0.5, 0.4, 0.6],
            vec![0.5, 0.4, 0.6],
            vec![0.5, 0.4, 0.6],
        ]
    }

    #[test]
    fn two_worlds_choose_as_the_halves_and_k_cuts_the_candidates() {
        let w = worlds();
        // Even worlds: pass +0.05 mean? (0.2 - 0.1)/2 = 0.05 < land 0.1: land, judged +0.1.
        assert!((lead_halves(&w, 3) - 0.1).abs() < 1e-12);
        // Without the land: even worlds choose the pass (+0.05), judged on 1 and 3: -0.1;
        // odd worlds keep the own answer.
        assert!((lead_halves(&w, 2) + 0.05).abs() < 1e-12);
        assert_eq!(lead_halves(&w, 1), 0.0);
    }

    #[test]
    fn one_world_chooses_and_three_judge_in_every_rotation() {
        let w = worlds();
        // World 0 chooses the pass (+0.2), judged on 1..3: -0.1; worlds 1..3 choose the land,
        // judged +0.1 each.
        assert!((lead_single(&w, 3) - (-0.1 + 0.3) / 4.0).abs() < 1e-12);
        // Without the land, only world 0 leaves the own answer.
        assert!((lead_single(&w, 2) + 0.1 / 4.0).abs() < 1e-12);
    }

    fn row(half: Half, game: u32, class: usize, score: f64, lead: f64) -> Row {
        Row {
            half,
            game: (0, game),
            class,
            score: Some(score),
            full: 0.1,
            leads: [[lead; 2]; 4],
        }
    }

    #[test]
    fn a_class_gate_takes_the_best_positive_classes_within_its_share() {
        let rows = vec![
            row(Half::Selection, 0, 0, 0., 0.3),
            row(Half::Selection, 0, 1, 0., 0.1),
            row(Half::Selection, 4, 1, 0., 0.1),
            row(Half::Selection, 4, 2, 0., -0.2),
            row(Half::Selection, 8, 4, 0., 0.05),
            row(Half::Report, 12, 0, 0., 0.3),
        ];
        let (searched, fitted) = fit_gate(&rows, Gate::Class, 0.5, 0, 0);
        // pass (1 of 5) then tap (2 of 5): 3/5 > 0.5, so tap is skipped and cast (1) taken.
        assert_eq!(fitted["classes"], json!(["pass", "cast"]));
        assert_eq!(searched, vec![true, false, false, false, true, true]);
        let (_, all) = fit_gate(&rows, Gate::Class, 1.0, 0, 0);
        assert_eq!(
            all["classes"],
            json!(["pass", "tap", "cast"]),
            "never the class that loses"
        );
    }

    #[test]
    fn a_score_gate_searches_its_share_of_the_selection_half_by_threshold() {
        let rows: Vec<Row> = (0..10)
            .map(|i| {
                row(
                    if i < 8 { Half::Selection } else { Half::Report },
                    i,
                    0,
                    f64::from(i),
                    0.1,
                )
            })
            .collect();
        let (searched, fitted) = fit_gate(&rows, Gate::Score(0), 0.25, 0, 0);
        // ceil(0.25 * 8) = 2 of the selection half: scores 7 and 6.
        assert_eq!(fitted["threshold"], json!(6.0));
        assert_eq!(
            searched.iter().filter(|s| **s).count(),
            4,
            "6, 7 and the report half's 8, 9"
        );
    }

    #[test]
    fn the_standard_error_is_by_game() {
        let values = [((0, 1), 0.1), ((0, 1), 0.3), ((0, 2), 0.0)];
        let (mean, se) = per_100(&values);
        assert!((mean - 40.0 / 3.0).abs() < 1e-9);
        // Game sums 0.4 and 0.0: variance 0.08, sqrt(2 * 0.08) / 3 * 100.
        assert!((se.unwrap() - (0.16_f64).sqrt() / 3.0 * 100.0).abs() < 1e-9);
    }

    #[test]
    fn the_chosen_configuration_is_eligible_and_the_best_on_the_selection_half() {
        let cell = |gate: &str, k: u32, p: u32, share: f64, kept: f64| {
            json!({"gate": gate, "K": k, "P": p,
            "selection": {"kept": kept, "share": share, "cost": f64::from(k) * f64::from(p) * share}, "report": {"kept": 0.7, "z": 2.5}})
        };
        let cells = vec![
            cell("class", 2, 1, 0.1, 0.5),
            cell("class", 3, 2, 0.3, 0.8),
            cell("class", 4, 2, 0.3, 0.9),
            cell("class", 2, 2, 0.5, 0.95),
            cell("class", 3, 1, 0.2, 0.8),
            cell("policy_margin", 2, 1, 0.1, 0.99),
        ];
        let chosen = choose_configuration(&cells, &["class"]).unwrap();
        assert_eq!(
            (chosen["K"].as_u64(), chosen["P"].as_u64()),
            (Some(3), Some(1)),
            "a tie goes to the cheaper"
        );
        assert_eq!(verdict(chosen)["passed"], json!(true));
    }

    #[test]
    fn combat_and_priority_roots_are_told_apart() {
        assert_eq!(class_of(&json!("PassPriority")), Some(0));
        assert_eq!(class_of(&json!({"PlayLand": {"card": 3}})), Some(2));
        assert_eq!(
            class_of(&json!({"DeclareAttackers": {"attackers": []}})),
            Some(5)
        );
        assert_eq!(class_of(&json!({"ChooseTargets": {}})), None);
    }

    fn event(n: u64, worlds: &[Vec<f64>]) -> Event {
        serde_json::from_value(json!({"accepted": true, "seat": 1, "record_input_n": n,
            "candidates": ["PassPriority", {"PlayLand": {"card": 3}}], "worlds": worlds, "chosen": 0}))
        .unwrap()
    }

    #[test]
    fn events_join_the_sidecar_in_order_and_only_where_they_agree() {
        let first = vec![vec![0.5, 0.6], vec![0.5, 0.4]];
        let second = vec![vec![0.5, 0.5], vec![0.5, 0.7]];
        #[allow(clippy::cast_possible_truncation)]
        let side = |cursor: u32, w: &[Vec<f64>]| -> SideRoot {
            (7, 1, cursor, 0, vec![0.0, paired(w, 1).0 as f32])
        };
        let sidecar = vec![side(90, &second), side(40, &first)];
        // Events out of order: the later input answers the later root.
        let joined = join(
            vec![(7, event(12, &second)), (7, event(5, &first))],
            &sidecar,
            0,
        )
        .unwrap();
        assert_eq!(
            joined.iter().map(|r| r.cursor).collect::<Vec<_>>(),
            vec![40, 90]
        );
        assert_eq!(joined[0].worlds, first);
        // Swapped worlds disagree with the sidecar's advantages.
        assert!(
            join(
                vec![(7, event(5, &second)), (7, event(12, &first))],
                &sidecar,
                0
            )
            .is_err()
        );
        // One event short.
        assert!(join(vec![(7, event(5, &first))], &sidecar, 0).is_err());
    }
}
