//! Dynamic Swiss Duplicate Tournament runner (`llm_collector4 swiss`).
//!
//! Pairs `2k` profiles over `R` Swiss rounds (`R < 2k`, zero repeat opponents).
//! Every match plays one seed- and deck-mirrored 2-leg Constructed deal under
//! tournament rules (`starting_life = 20`, `0` free mulligans, `0` House-AI
//! fallback tolerance) and scores **3 Match Points** for a deal win (`2-0` or
//! `1.5-0.5`), **1 Match Point** for a `1-1` split, and **0 Match Points** for
//! a deal loss, ranked by `MP` -> `OMW%` (Buchholz, `0.33` floor) -> `GW%` ->
//! `OGW%` -> Bradley-Terry Elo.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use anyhow::{Context as _, Result, bail, ensure};
use baylee_seat::config::{Overrides, Paths, SeatSettings, plan};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use crate::audit::write_atomic_json;
use crate::compare::{Pairs, Tally, add_game, bradley_terry_ladder};
use crate::controller_lock;
use crate::duplicate::{
    EvalDeck, LegExecParams, LegSpec, TableRules, coprime_stride, load_constructed_decks,
    run_single_leg_with_rules, splitmix64,
};

/// Configuration for `llm_collector4 swiss`.
#[derive(Clone, Debug)]
pub struct SwissConfig {
    pub root: PathBuf,
    pub profiles_config: PathBuf,
    pub profiles: Vec<String>,
    pub decks_dir: PathBuf,
    pub rounds: usize,
    pub base_seed: u64,
    pub concurrency: usize,
    pub call_timeout_seconds: u64,
    pub max_actions: u32,
    pub max_leg_retries: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct RoundMatchSpec {
    round: usize,
    table: usize,
    deal_index: usize,
    deal_id: String,
    seed: u64,
    pair: [String; 2],
    decks: [String; 2],
}

#[derive(Clone, Debug, Serialize)]
struct MatchOutcome {
    round: usize,
    table: usize,
    deal_id: String,
    seed_hex: String,
    pair: [String; 2],
    decks: [String; 2],
    leg0_winner: Option<String>,
    leg1_winner: Option<String>,
    half_points: [u32; 2],
    match_points: [u32; 2],
    house_answers: [u64; 2],
    actions: [u64; 2],
    reports: [u64; 2],
}

#[derive(Clone, Debug, Serialize)]
struct StandingRow {
    rank: usize,
    profile: String,
    rounds_played: u32,
    match_points: u32,
    matches_won: u32,
    matches_drawn: u32,
    matches_lost: u32,
    half_points: u32,
    deal_points: f64,
    mw_pct: f64,
    omw_pct: f64,
    gw_pct: f64,
    ogw_pct: f64,
    elo: f64,
    elo_ci95: f64,
    games: u64,
    game_wins: u64,
    game_draws: u64,
    answers: u64,
    ms_per_ans: Option<f64>,
    tok_per_ans: Option<f64>,
    house_answers: u64,
    opponents: Vec<String>,
}

/// Runs a complete `R`-round Swiss Duplicate Tournament under `cfg.root`.
pub async fn run(cfg: SwissConfig) -> Result<()> {
    validate_config(&cfg)?;
    let _lock = controller_lock(&cfg.root)?;
    std::fs::create_dir_all(cfg.root.join("attempts"))?;

    let settings_file = load_and_verify_profiles(&cfg)?;
    let decks = load_constructed_decks(&cfg.decks_dir)?;
    ensure!(
        decks.len() >= 2,
        "need at least 2 Constructed decks in {}",
        cfg.decks_dir.display()
    );
    let deck_by_key: BTreeMap<String, EvalDeck> =
        decks.iter().map(|d| (d.key.clone(), d.clone())).collect();
    let deck_pairs = all_deck_pairs(&decks);

    eprintln!(
        "[swiss] {} profiles, {} rounds ({} deals / {} games), {} Constructed decks, concurrency={}, strict_zero_house=true",
        cfg.profiles.len(),
        cfg.rounds,
        cfg.rounds * (cfg.profiles.len() / 2),
        cfg.rounds * cfg.profiles.len(),
        decks.len(),
        cfg.concurrency,
    );

    let cfg_arc = Arc::new(cfg);
    let settings_arc = Arc::new(settings_file);
    let mut completed_matches: Vec<MatchOutcome> = Vec::new();
    let mut played_pairs: BTreeSet<(usize, usize)> = BTreeSet::new();

    for round in 1..=cfg_arc.rounds {
        let standings = compute_standings(&cfg_arc.root, &cfg_arc.profiles, &completed_matches);
        let round_specs = resolve_round_specs(
            &cfg_arc,
            round,
            &standings,
            &played_pairs,
            &decks,
            &deck_pairs,
        )?;
        for spec in &round_specs {
            if let (Some(ia), Some(ib)) = (
                profile_index(&cfg_arc.profiles, &spec.pair[0]),
                profile_index(&cfg_arc.profiles, &spec.pair[1]),
            ) {
                played_pairs.insert((ia.min(ib), ia.max(ib)));
            }
        }
        execute_round_legs(&cfg_arc, &settings_arc, &round_specs, &deck_by_key).await?;
        let outcomes = collect_round_outcomes(&cfg_arc.root, &round_specs)?;
        completed_matches.extend(outcomes);

        let updated = compute_standings(&cfg_arc.root, &cfg_arc.profiles, &completed_matches);
        write_tournament_outputs(&cfg_arc, round, &updated, &completed_matches)?;
        eprintln!(
            "[swiss] === Round {round}/{} complete — leader: {} ({} MP, {:.1}% OMW) ===",
            cfg_arc.rounds,
            updated[0].profile,
            updated[0].match_points,
            100.0 * updated[0].omw_pct,
        );
    }
    Ok(())
}

fn validate_config(cfg: &SwissConfig) -> Result<()> {
    ensure!(cfg.concurrency >= 1, "--concurrency must be >= 1");
    ensure!(
        cfg.profiles.len() >= 2 && cfg.profiles.len().is_multiple_of(2),
        "--profiles must contain an even number of profiles (>= 2), got {}",
        cfg.profiles.len()
    );
    let unique: BTreeSet<&str> = cfg.profiles.iter().map(String::as_str).collect();
    ensure!(
        unique.len() == cfg.profiles.len(),
        "--profiles contains duplicate profile names"
    );
    ensure!(
        cfg.rounds >= 1 && cfg.rounds < cfg.profiles.len(),
        "--rounds must be in 1..{}, got {}",
        cfg.profiles.len(),
        cfg.rounds
    );
    Ok(())
}

fn load_and_verify_profiles(cfg: &SwissConfig) -> Result<SeatSettings> {
    let sys_env = |key: &str| std::env::var(key).ok();
    let paths = Paths::resolve(Some(&cfg.profiles_config), None, &sys_env);
    let settings_file = paths
        .load()
        .map_err(|e| anyhow::anyhow!("{e}"))?
        .context("loading --profiles-config")?;
    for prof in &cfg.profiles {
        plan(
            None,
            Some(&settings_file),
            &paths,
            Some(prof),
            &Overrides::default(),
        )
        .map_err(|e| anyhow::anyhow!("invalid profile {prof}: {e}"))?
        .with_context(|| format!("profile {prof} resolved to house instead of LLM"))?;
    }
    Ok(settings_file)
}

fn all_deck_pairs(decks: &[EvalDeck]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    for i in 0..decks.len() {
        for j in (i + 1)..decks.len() {
            out.push((i, j));
        }
    }
    out
}

fn profile_index(profiles: &[String], target: &str) -> Option<usize> {
    profiles.iter().position(|p| p == target)
}

fn resolve_round_specs(
    cfg: &SwissConfig,
    round: usize,
    standings: &[StandingRow],
    played_pairs: &BTreeSet<(usize, usize)>,
    decks: &[EvalDeck],
    deck_pairs: &[(usize, usize)],
) -> Result<Vec<RoundMatchSpec>> {
    let path = cfg
        .root
        .join(format!("swiss_round_{round:02}_pairings.json"));
    if let Ok(raw) = std::fs::read(&path) {
        let specs: Vec<RoundMatchSpec> =
            serde_json::from_slice(&raw).with_context(|| format!("parsing {}", path.display()))?;
        if specs.len() == cfg.profiles.len() / 2 {
            eprintln!(
                "[swiss] Loaded locked Round {round} pairings from {}",
                path.display()
            );
            return Ok(specs);
        }
    }

    let pairs = pair_swiss_round(&cfg.profiles, standings, round, played_pairs)?;
    let tables = cfg.profiles.len() / 2;
    let stride = coprime_stride(deck_pairs.len());
    let mut specs = Vec::with_capacity(tables);
    for (idx, (ia, ib)) in pairs.into_iter().enumerate() {
        let deal_index = (round - 1) * tables + idx;
        let seed = splitmix64(cfg.base_seed.wrapping_add(deal_index as u64));
        let (d0, d1) = deck_pairs[(deal_index * stride + 5) % deck_pairs.len()];
        specs.push(RoundMatchSpec {
            round,
            table: idx + 1,
            deal_index,
            deal_id: format!("round{round:02}-table{:02}", idx + 1),
            seed,
            pair: [cfg.profiles[ia].clone(), cfg.profiles[ib].clone()],
            decks: [decks[d0].key.clone(), decks[d1].key.clone()],
        });
    }
    write_atomic_json(&path, &serde_json::to_value(&specs)?)?;
    Ok(specs)
}

/// Pairs one Swiss round with zero repeat opponents.
///
/// Round 1 pairs cross-half (`i` vs `i + N/2`) so multi-family profile lists
/// immediately play cross-family matches; subsequent rounds find the exact
/// minimum-cost zero-rematch perfect matching over current Swiss rank order,
/// prioritizing equal Match-Point brackets and adjacent standings.
fn pair_swiss_round(
    profiles: &[String],
    standings: &[StandingRow],
    round: usize,
    played: &BTreeSet<(usize, usize)>,
) -> Result<Vec<(usize, usize)>> {
    let n = profiles.len();
    if round == 1 {
        let half = n / 2;
        return Ok((0..half).map(|i| (i, i + half)).collect());
    }
    let ordered: Vec<(usize, u32)> = standings
        .iter()
        .filter_map(|row| profile_index(profiles, &row.profile).map(|idx| (idx, row.match_points)))
        .collect();
    ensure!(ordered.len() == n, "standings size mismatch");

    let mut used = vec![false; n];
    let mut current = Vec::with_capacity(n / 2);
    let mut best: Option<(u64, Vec<(usize, usize)>)> = None;
    search_min_cost_matching(&ordered, played, &mut used, 0, &mut current, &mut best);
    let (_, chosen) = best.context("no valid zero-rematch Swiss pairing exists")?;
    Ok(chosen)
}

fn search_min_cost_matching(
    ordered: &[(usize, u32)],
    played: &BTreeSet<(usize, usize)>,
    used: &mut [bool],
    cost_so_far: u64,
    current: &mut Vec<(usize, usize)>,
    best: &mut Option<(u64, Vec<(usize, usize)>)>,
) {
    if let Some((best_cost, _)) = best
        && cost_so_far >= *best_cost
    {
        return;
    }
    let Some(first_rank) = used.iter().position(|&u| !u) else {
        *best = Some((cost_so_far, current.clone()));
        return;
    };
    used[first_rank] = true;
    let (pa, mp_a) = ordered[first_rank];

    let mut candidates: Vec<(u64, usize, usize)> = ((first_rank + 1)..ordered.len())
        .filter(|&r| !used[r])
        .filter_map(|r| {
            let (pb, mp_b) = ordered[r];
            let key = (pa.min(pb), pa.max(pb));
            if played.contains(&key) {
                return None;
            }
            let mp_diff = u64::from(mp_a.abs_diff(mp_b));
            let rank_diff = (r - first_rank) as u64;
            let edge_cost = mp_diff * 10_000 + rank_diff * rank_diff;
            Some((edge_cost, r, pb))
        })
        .collect();
    candidates.sort_unstable_by_key(|&(c, r, _)| (c, r));

    for (edge_cost, r, pb) in candidates {
        used[r] = true;
        current.push((pa, pb));
        search_min_cost_matching(
            ordered,
            played,
            used,
            cost_so_far + edge_cost,
            current,
            best,
        );
        current.pop();
        used[r] = false;
    }
    used[first_rank] = false;
}

async fn execute_round_legs(
    cfg: &Arc<SwissConfig>,
    settings_file: &Arc<SeatSettings>,
    specs: &[RoundMatchSpec],
    deck_by_key: &BTreeMap<String, EvalDeck>,
) -> Result<()> {
    let legs = build_round_legs(specs, deck_by_key)?;
    let sem = Arc::new(Semaphore::new(cfg.concurrency));
    let halted = Arc::new(AtomicBool::new(false));
    let stop_file = cfg.root.join("STOP");
    let drain_file = cfg.root.join("DRAIN");
    let mut tasks = JoinSet::new();

    for leg in legs {
        while let Some(joined) = tasks.try_join_next() {
            joined.context("swiss worker panicked")??;
        }
        if halted.load(Ordering::SeqCst) || stop_file.exists() || drain_file.exists() {
            bail!("STOP/DRAIN or unrecoverable leg failure halted Swiss round");
        }
        let attempt_dir = cfg
            .root
            .join("attempts")
            .join(format!("attempt-{:09}", leg.attempt_index));
        if admitted_strict_receipt(&attempt_dir, &leg).is_some() {
            eprintln!(
                "[swiss] SKIP attempt-{:09} ({} leg {} already strictly admitted)",
                leg.attempt_index, leg.deal_id, leg.leg
            );
            continue;
        }
        let permit = Arc::clone(&sem).acquire_owned().await?;
        let cfg_cl = Arc::clone(cfg);
        let file_cl = Arc::clone(settings_file);
        let halted_cl = Arc::clone(&halted);
        tasks.spawn(async move {
            let _permit = permit;
            let res = run_leg_with_retries(&cfg_cl, &file_cl, &leg, &attempt_dir).await;
            if res.is_err() {
                halted_cl.store(true, Ordering::SeqCst);
            }
            res
        });
    }

    while let Some(joined) = tasks.join_next().await {
        joined.context("swiss worker panicked")??;
    }
    Ok(())
}

fn build_round_legs(
    specs: &[RoundMatchSpec],
    deck_by_key: &BTreeMap<String, EvalDeck>,
) -> Result<Vec<LegSpec>> {
    let mut legs = Vec::with_capacity(specs.len() * 2);
    for spec in specs {
        let d0 = deck_by_key
            .get(&spec.decks[0])
            .with_context(|| format!("unknown deck {}", spec.decks[0]))?;
        let d1 = deck_by_key
            .get(&spec.decks[1])
            .with_context(|| format!("unknown deck {}", spec.decks[1]))?;
        let [pa, pb] = &spec.pair;
        for (leg, seat_profiles) in [
            (0_u8, [pa.clone(), pb.clone()]),
            (1_u8, [pb.clone(), pa.clone()]),
        ] {
            legs.push(LegSpec {
                attempt_index: spec.deal_index * 2 + usize::from(leg),
                deal_index: spec.deal_index,
                deal_id: spec.deal_id.clone(),
                leg,
                seed: spec.seed,
                pair_profiles: [pa.clone(), pb.clone()],
                seat_profiles,
                decks: [d0.clone(), d1.clone()],
                swiss_round: Some(spec.round),
                swiss_table: Some(spec.table),
            });
        }
    }
    Ok(legs)
}

async fn run_leg_with_retries(
    cfg: &SwissConfig,
    settings_file: &SeatSettings,
    leg: &LegSpec,
    attempt_dir: &Path,
) -> Result<()> {
    const BACKOFF_SECS: [u64; 6] = [5, 15, 30, 60, 90, 120];
    let exec = LegExecParams {
        profiles_config: cfg.profiles_config.clone(),
        call_timeout_seconds: cfg.call_timeout_seconds,
        max_actions: cfg.max_actions,
        rules: TableRules {
            tournament_constructed: true,
            strict_zero_house: true,
        },
    };
    let stop_file = cfg.root.join("STOP");
    let drain_file = cfg.root.join("DRAIN");

    for attempt_no in 0..=cfg.max_leg_retries {
        if stop_file.exists() || drain_file.exists() {
            bail!("STOP/DRAIN detected before {} leg {}", leg.deal_id, leg.leg);
        }
        match run_single_leg_with_rules(&exec, settings_file, leg, attempt_dir).await {
            Ok(()) if admitted_strict_receipt(attempt_dir, leg).is_some() => return Ok(()),
            Ok(()) => bail!(
                "receipt verification failed after {} leg {}",
                leg.deal_id,
                leg.leg
            ),
            Err(err) if attempt_no < cfg.max_leg_retries => {
                let wait = BACKOFF_SECS[attempt_no.min(BACKOFF_SECS.len() - 1)];
                eprintln!(
                    "[swiss] RETRY {}/{} for attempt-{:09} ({} leg {} [{} vs {}]) in {wait}s after: {err:#}",
                    attempt_no + 1,
                    cfg.max_leg_retries,
                    leg.attempt_index,
                    leg.deal_id,
                    leg.leg,
                    leg.seat_profiles[0],
                    leg.seat_profiles[1],
                );
                for _ in 0..wait {
                    if stop_file.exists() || drain_file.exists() {
                        bail!("STOP/DRAIN detected during retry backoff");
                    }
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
            Err(err) => {
                return Err(err).with_context(|| {
                    format!(
                        "exhausted {} retries for {} leg {}",
                        cfg.max_leg_retries, leg.deal_id, leg.leg
                    )
                });
            }
        }
    }
    bail!("unreachable retry loop exit")
}

fn admitted_strict_receipt(attempt_dir: &Path, leg: &LegSpec) -> Option<Value> {
    let raw = std::fs::read(attempt_dir.join("receipt.json")).ok()?;
    let receipt: Value = serde_json::from_slice(&raw).ok()?;
    if receipt["status"].as_str() != Some("admitted") {
        return None;
    }
    if receipt["duplicate"]["deal_id"].as_str() != Some(&leg.deal_id)
        || receipt["duplicate"]["leg"].as_u64() != Some(u64::from(leg.leg))
    {
        return None;
    }
    for seat in 0..2 {
        let counts = &receipt["technical"]["seats"][seat]["transcript_answer_counts"];
        if counts["house"].as_u64() != Some(0) || counts["other"].as_u64() != Some(0) {
            return None;
        }
    }
    Some(receipt)
}

fn collect_round_outcomes(root: &Path, specs: &[RoundMatchSpec]) -> Result<Vec<MatchOutcome>> {
    let mut out = Vec::with_capacity(specs.len());
    for spec in specs {
        let r0 = read_admitted_leg(root, spec.deal_index * 2)?;
        let r1 = read_admitted_leg(root, spec.deal_index * 2 + 1)?;
        let (pts0_a, win0) = score_leg_for_pair(&r0, &spec.pair, 0);
        let (pts1_a, win1) = score_leg_for_pair(&r1, &spec.pair, 1);
        let total_a = pts0_a + pts1_a;
        let total_b = 4 - total_a;
        let mp = match total_a.cmp(&2) {
            std::cmp::Ordering::Greater => [3, 0],
            std::cmp::Ordering::Equal => [1, 1],
            std::cmp::Ordering::Less => [0, 3],
        };
        let house_a = seat_house(&r0, 0) + seat_house(&r1, 1);
        let house_b = seat_house(&r0, 1) + seat_house(&r1, 0);
        ensure!(
            house_a == 0 && house_b == 0,
            "non-zero house fallback in {} ({house_a}, {house_b})",
            spec.deal_id
        );
        out.push(MatchOutcome {
            round: spec.round,
            table: spec.table,
            deal_id: spec.deal_id.clone(),
            seed_hex: format!("{:#018x}", spec.seed),
            pair: spec.pair.clone(),
            decks: spec.decks.clone(),
            leg0_winner: win0,
            leg1_winner: win1,
            half_points: [total_a, total_b],
            match_points: mp,
            house_answers: [house_a, house_b],
            actions: [
                r0["technical"]["actions"].as_u64().unwrap_or(0),
                r1["technical"]["actions"].as_u64().unwrap_or(0),
            ],
            reports: [
                r0["technical"]["llm_reports"].as_u64().unwrap_or(0),
                r1["technical"]["llm_reports"].as_u64().unwrap_or(0),
            ],
        });
    }
    Ok(out)
}

fn read_admitted_leg(root: &Path, attempt_index: usize) -> Result<Value> {
    let path = root
        .join("attempts")
        .join(format!("attempt-{attempt_index:09}"))
        .join("receipt.json");
    let raw = std::fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
    let val: Value = serde_json::from_slice(&raw)?;
    ensure!(
        val["status"].as_str() == Some("admitted"),
        "{} is not admitted",
        path.display()
    );
    Ok(val)
}

fn score_leg_for_pair(receipt: &Value, pair: &[String; 2], a_seat: u64) -> (u32, Option<String>) {
    let term = &receipt["technical"]["terminal"];
    if term["draw"].as_bool().unwrap_or(false) {
        return (1, None);
    }
    match term["winner_seat"].as_u64() {
        Some(w) if w == a_seat => (2, Some(pair[0].clone())),
        Some(_) => (0, Some(pair[1].clone())),
        None => (1, None),
    }
}

fn seat_house(receipt: &Value, seat: usize) -> u64 {
    receipt["technical"]["seats"][seat]["transcript_answer_counts"]["house"]
        .as_u64()
        .unwrap_or(0)
}

#[derive(Default)]
struct RawPlayerAcc {
    rounds: u32,
    mp: u32,
    won: u32,
    drawn: u32,
    lost: u32,
    half_pts: u32,
    house_answers: u64,
    opponents: Vec<String>,
}

fn compute_standings(
    root: &Path,
    profiles: &[String],
    matches: &[MatchOutcome],
) -> Vec<StandingRow> {
    let mut acc: BTreeMap<String, RawPlayerAcc> = profiles
        .iter()
        .map(|p| (p.clone(), RawPlayerAcc::default()))
        .collect();
    for m in matches {
        for side in 0..2 {
            let me = &m.pair[side];
            let opp = &m.pair[1 - side];
            let entry = acc.entry(me.clone()).or_default();
            entry.rounds += 1;
            entry.mp += m.match_points[side];
            entry.half_pts += m.half_points[side];
            entry.house_answers += m.house_answers[side];
            entry.opponents.push(opp.clone());
            match m.match_points[side] {
                3 => entry.won += 1,
                1 => entry.drawn += 1,
                _ => entry.lost += 1,
            }
        }
    }

    let (elo_map, tallies) = load_elo_and_tallies(root, matches.len() * 2);
    build_ranked_rows(profiles, &acc, &elo_map, &tallies)
}

fn build_ranked_rows(
    profiles: &[String],
    acc: &BTreeMap<String, RawPlayerAcc>,
    elo_map: &BTreeMap<String, (f64, f64)>,
    tallies: &BTreeMap<String, Tally>,
) -> Vec<StandingRow> {
    let mw_of = |p: &str| -> f64 {
        let a = &acc[p];
        if a.rounds == 0 {
            0.33
        } else {
            (f64::from(a.mp) / f64::from(a.rounds * 3)).max(0.33)
        }
    };
    let gw_of = |p: &str| -> f64 {
        let a = &acc[p];
        if a.rounds == 0 {
            0.33
        } else {
            (f64::from(a.half_pts) / f64::from(a.rounds * 4)).max(0.33)
        }
    };

    let mut rows: Vec<StandingRow> = profiles
        .iter()
        .map(|prof| {
            let a = &acc[prof];
            let n_opp = u32::try_from(a.opponents.len()).unwrap_or(0);
            let omw = if n_opp == 0 {
                0.0
            } else {
                a.opponents.iter().map(|o| mw_of(o)).sum::<f64>() / f64::from(n_opp)
            };
            let ogw = if n_opp == 0 {
                0.0
            } else {
                a.opponents.iter().map(|o| gw_of(o)).sum::<f64>() / f64::from(n_opp)
            };
            let (elo, elo_ci95) = elo_map.get(prof).copied().unwrap_or((1500.0, 0.0));
            let t = tallies.get(prof).cloned().unwrap_or_default();
            let ans_f = f64::from(u32::try_from(t.answers).unwrap_or(u32::MAX));
            StandingRow {
                rank: 0,
                profile: prof.clone(),
                rounds_played: a.rounds,
                match_points: a.mp,
                matches_won: a.won,
                matches_drawn: a.drawn,
                matches_lost: a.lost,
                half_points: a.half_pts,
                deal_points: 0.5 * f64::from(a.half_pts),
                mw_pct: mw_of(prof),
                omw_pct: omw,
                gw_pct: gw_of(prof),
                ogw_pct: ogw,
                elo,
                elo_ci95,
                games: t.games,
                game_wins: t.wins,
                game_draws: t.draws,
                answers: t.answers,
                ms_per_ans: (t.answers > 0)
                    .then(|| f64::from(u32::try_from(t.think_ms).unwrap_or(u32::MAX)) / ans_f),
                tok_per_ans: (t.answers > 0)
                    .then(|| f64::from(u32::try_from(t.tokens).unwrap_or(u32::MAX)) / ans_f),
                house_answers: a.house_answers,
                opponents: a.opponents.clone(),
            }
        })
        .collect();

    rows.sort_by(|a, b| {
        b.match_points
            .cmp(&a.match_points)
            .then_with(|| b.omw_pct.total_cmp(&a.omw_pct))
            .then_with(|| b.gw_pct.total_cmp(&a.gw_pct))
            .then_with(|| b.ogw_pct.total_cmp(&a.ogw_pct))
            .then_with(|| b.half_points.cmp(&a.half_points))
            .then_with(|| b.elo.total_cmp(&a.elo))
            .then_with(|| {
                profile_index(profiles, &a.profile).cmp(&profile_index(profiles, &b.profile))
            })
    });
    for (i, row) in rows.iter_mut().enumerate() {
        row.rank = i + 1;
    }
    rows
}

fn load_elo_and_tallies(
    root: &Path,
    admitted_legs_limit: usize,
) -> (BTreeMap<String, (f64, f64)>, BTreeMap<String, Tally>) {
    let mut pairs = Pairs::new();
    for idx in 0..admitted_legs_limit {
        let dir = root.join("attempts").join(format!("attempt-{idx:09}"));
        let Ok(raw) = std::fs::read(dir.join("receipt.json")) else {
            continue;
        };
        let Ok(receipt) = serde_json::from_slice::<Value>(&raw) else {
            continue;
        };
        if receipt["status"] == "admitted" {
            add_game(&dir, &receipt, &mut pairs);
        }
    }
    let mut totals_ref: BTreeMap<&str, (Tally, Tally)> = BTreeMap::new();
    let mut tallies: BTreeMap<String, Tally> = BTreeMap::new();
    for ((me, _), tally) in &pairs {
        totals_ref.entry(me.as_str()).or_default().0.add(tally);
        tallies.entry(me.clone()).or_default().add(tally);
    }
    let ladder = bradley_terry_ladder(&pairs, &totals_ref);
    let elo_map = ladder
        .into_iter()
        .map(|row| (row.profile.to_owned(), (row.elo, row.ci95)))
        .collect();
    (elo_map, tallies)
}

fn write_tournament_outputs(
    cfg: &SwissConfig,
    completed_rounds: usize,
    standings: &[StandingRow],
    matches: &[MatchOutcome],
) -> Result<()> {
    let total_house: u64 = standings.iter().map(|r| r.house_answers).sum();
    let payload = json!({
        "tournament": "Swiss Duplicate Magic (Constructed, Seed- & Deck-Mirrored)",
        "completed_rounds": completed_rounds,
        "target_rounds": cfg.rounds,
        "profiles_count": cfg.profiles.len(),
        "total_matches": matches.len(),
        "total_games": matches.len() * 2,
        "total_house_ai_answers": total_house,
        "strict_zero_house_verified": total_house == 0,
        "standings": standings,
        "matches": matches,
    });
    write_atomic_json(&cfg.root.join("swiss_standings.json"), &payload)?;
    let md = render_markdown(cfg, completed_rounds, standings, matches);
    std::fs::write(cfg.root.join("swiss_standings.md"), md)?;
    Ok(())
}

fn render_markdown(
    cfg: &SwissConfig,
    completed_rounds: usize,
    standings: &[StandingRow],
    matches: &[MatchOutcome],
) -> String {
    let mut out = String::new();
    let total_house: u64 = standings.iter().map(|r| r.house_answers).sum();
    let _ = writeln!(
        out,
        "# Swiss Duplicate Tournament ({completed_rounds}/{} Runden, {} Partien)\n",
        cfg.rounds,
        matches.len() * 2,
    );
    let _ = writeln!(
        out,
        "- **Modus:** Schweizer System mit gespiegelten 2-Leg Duplicate-Deals (`starting_life = 20`, `0` freie Mulligans)\n\
         - **Wertung:** `3 MP` Deal-Sieg (`2-0` / `1.5-0.5`), `1 MP` Split (`1-1`), `0 MP` Deal-Niederlage\n\
         - **Tiebreaker:** `MP` → `OMW%` (Buchholz, 33%-Floor) → `GW%` → `OGW%` → `Bradley-Terry Elo`\n\
         - **Haus-KI-Antworten gesamt:** `{total_house}` (strikte `0`-Toleranz)\n"
    );
    let _ = writeln!(
        out,
        "| Platz | Profil | MP | S-U-N | Deal-Pkt | OMW% | GW% | OGW% | BT-Elo (±95%) | ms/ans | tok/ans | Haus-KI |\n\
         |---:|---|---:|:---:|---:|---:|---:|---:|---:|---:|---:|---:|"
    );
    for r in standings {
        let ms = r
            .ms_per_ans
            .map_or_else(|| "-".into(), |v| format!("{v:.0}"));
        let tok = r
            .tok_per_ans
            .map_or_else(|| "-".into(), |v| format!("{v:.0}"));
        let _ = writeln!(
            out,
            "| **{}** | `{}` | **{}** | {}-{}-{} | {:.1}/{:.1} | {:.1}% | {:.1}% | {:.1}% | {:.0} (±{:.0}) | {} | {} | {} |",
            r.rank,
            r.profile,
            r.match_points,
            r.matches_won,
            r.matches_drawn,
            r.matches_lost,
            r.deal_points,
            f64::from(r.rounds_played * 2),
            100.0 * r.omw_pct,
            100.0 * r.gw_pct,
            100.0 * r.ogw_pct,
            r.elo,
            r.elo_ci95,
            ms,
            tok,
            r.house_answers,
        );
    }
    let _ = writeln!(out, "\n## Runden & Tische\n");
    let _ = writeln!(
        out,
        "| Runde | Tisch | Paarung (`A` vs `B`) | Decks (`Deck 0` vs `Deck 1`) | Leg 0 | Leg 1 | Deal | MP |\n\
         |---:|---:|---|---|---|---|:---:|:---:|"
    );
    for m in matches {
        let l0 = m.leg0_winner.as_deref().unwrap_or("Draw");
        let l1 = m.leg1_winner.as_deref().unwrap_or("Draw");
        let _ = writeln!(
            out,
            "| {} | {} | `{}` vs `{}` | `{}` vs `{}` | `{l0}` | `{l1}` | {:.1}–{:.1} | {}–{} |",
            m.round,
            m.table,
            m.pair[0],
            m.pair[1],
            m.decks[0],
            m.decks[1],
            0.5 * f64::from(m.half_points[0]),
            0.5 * f64::from(m.half_points[1]),
            m.match_points[0],
            m.match_points[1],
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn accumulate(profiles: &[String], matches: &[MatchOutcome]) -> BTreeMap<String, RawPlayerAcc> {
        let mut acc: BTreeMap<String, RawPlayerAcc> = profiles
            .iter()
            .map(|p| (p.clone(), RawPlayerAcc::default()))
            .collect();
        for m in matches {
            for side in 0..2 {
                let e = acc.get_mut(&m.pair[side]).unwrap();
                e.rounds += 1;
                e.mp += m.match_points[side];
                e.half_pts += m.half_points[side];
                e.opponents.push(m.pair[1 - side].clone());
            }
        }
        acc
    }

    #[test]
    fn ten_profiles_complete_five_swiss_rounds_with_zero_rematches_and_exact_tiebreakers() {
        let profiles: Vec<String> = [
            "hydrogen-low",
            "hydrogen-medium",
            "hydrogen-high",
            "hydrogen-xhigh",
            "argon-low",
            "argon-medium",
            "argon-high",
            "carbon-low",
            "carbon-medium",
            "carbon-high",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();

        let mut played: BTreeSet<(usize, usize)> = BTreeSet::new();
        let mut matches: Vec<MatchOutcome> = Vec::new();
        let empty_elo = BTreeMap::new();
        let empty_tallies = BTreeMap::new();

        for round in 1..=5 {
            let acc = accumulate(&profiles, &matches);
            let standings = build_ranked_rows(&profiles, &acc, &empty_elo, &empty_tallies);
            let pairs = pair_swiss_round(&profiles, &standings, round, &played).unwrap();
            assert_eq!(pairs.len(), 5);

            let mut seen_in_round = BTreeSet::new();
            for (table_idx, (ia, ib)) in pairs.into_iter().enumerate() {
                assert_ne!(ia, ib);
                assert!(seen_in_round.insert(ia));
                assert!(seen_in_round.insert(ib));
                let key = (ia.min(ib), ia.max(ib));
                assert!(played.insert(key), "rematch detected in round {round}");

                // Deterministic synthetic strength: higher index wins, except table 2 splits 1-1.
                let (hp, mp) = if table_idx == 2 {
                    ([2, 2], [1, 1])
                } else if ia > ib {
                    ([4, 0], [3, 0])
                } else {
                    ([0, 4], [0, 3])
                };
                matches.push(MatchOutcome {
                    round,
                    table: table_idx + 1,
                    deal_id: format!("round{round:02}-table{:02}", table_idx + 1),
                    seed_hex: "0x1".into(),
                    pair: [profiles[ia].clone(), profiles[ib].clone()],
                    decks: ["d0".into(), "d1".into()],
                    leg0_winner: None,
                    leg1_winner: None,
                    half_points: hp,
                    match_points: mp,
                    house_answers: [0, 0],
                    actions: [100, 100],
                    reports: [0, 0],
                });
            }
        }

        assert_eq!(
            matches.len(),
            25,
            "5 rounds * 5 tables = 25 deals (50 legs)"
        );
        assert_eq!(played.len(), 25, "25 unique unordered pairings");

        let acc = accumulate(&profiles, &matches);
        let final_rows = build_ranked_rows(&profiles, &acc, &empty_elo, &empty_tallies);
        assert_eq!(final_rows.len(), 10);
        for (idx, r) in final_rows.iter().enumerate() {
            assert_eq!(r.rank, idx + 1);
            assert_eq!(r.rounds_played, 5);
            assert_eq!(r.opponents.len(), 5);
            assert!((0.33..=1.0).contains(&r.mw_pct));
            assert!((0.33..=1.0).contains(&r.omw_pct));
            assert!((0.33..=1.0).contains(&r.gw_pct));
            assert!((0.33..=1.0).contains(&r.ogw_pct));
        }
        assert!(final_rows.windows(2).all(|w| {
            (w[0].match_points, w[0].omw_pct.to_bits())
                >= (w[1].match_points, w[1].omw_pct.to_bits())
                || w[0].match_points > w[1].match_points
        }));
    }
}
