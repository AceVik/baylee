//! Group-relative weights for self-imitation from repeated deals.
//!
//! The same deals are played G times by the same policy with different
//! policy seeds (`eval4 --policy-seed`). A net seat's advantage in one game is
//! its outcome minus the mean outcome of the same deal and seat in the other
//! runs (leave-one-out). That removes the luck of the deal without a critic.
//! Only games that did better than their group are imitated, each weighted by
//! its advantage; `convert4 --game-weights` keeps exactly those net decisions.
//! House seats never receive a weight: the house AI is an opponent only.

use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::io::BufRead as _;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, bail, ensure};
use serde_json::{Value, json};

/// The weight file's contract name, checked by `convert4`.
pub const CONTRACT: &str = "group_self_imitation_v1";

/// One finished game of one net seat.
#[derive(Debug)]
struct Member {
    run: usize,
    record_sha256: String,
    seat: u8,
    /// Outcome in half points: 2 win, 1 draw, 0 loss.
    halves: u32,
}

/// The deal identity every run must agree on.
#[derive(Debug, PartialEq)]
struct Deal {
    seed: u64,
    focal_seat: u8,
}

/// Every net seat's finished games, grouped by deal and seat.
type Groups = BTreeMap<(u64, u8), Vec<Member>>;

fn run_identity(run: &Value) -> Value {
    // Everything that fixes the deals and the policy; the policy seed is in
    // each game line and must differ between runs.
    json!({
        "decks": run["decks"], "seats": run["seats"], "deal_offset": run["deal_offset"],
        "planned_deals": run["planned_deals"], "model_identity": run["model_identity"],
        "policy_mode": run["policy_mode"], "temperature": run["temperature"],
        "thinking_steps": run["thinking_steps"], "collection_contract": run["collection_contract"],
    })
}

/// A net seat's outcome in half points; `None` when it is unknown.
fn halves(game: &Value, seat: u8) -> Option<u32> {
    if game["capped"] == true || game["complete"] != true {
        return None;
    }
    let outcome = &game["outcome"];
    match outcome["kind"].as_str()? {
        "won" => Some(if outcome["seat"].as_u64()? == u64::from(seat) {
            2
        } else {
            0
        }),
        "draw" => Some(1),
        _ => None,
    }
}

/// The seats a net played in `game`: its learner seats when roles are
/// recorded, otherwise the focal seat of an evaluation against the house.
fn net_seats(game: &Value) -> Result<Vec<u8>> {
    if let Some(roles) = game["actor_roles"].as_array() {
        return Ok(roles
            .iter()
            .enumerate()
            .filter(|(_, role)| role.as_str() == Some("learner"))
            .map(|(seat, _)| seat as u8)
            .collect());
    }
    let focal = game["focal_seat"].as_u64().context("missing focal_seat")?;
    Ok(vec![u8::try_from(focal)?])
}

/// Reads every run's net games and checks that the runs repeat one deal set.
fn read_groups(runs: &[PathBuf]) -> Result<(Groups, Vec<String>)> {
    let mut identity = None;
    let mut policy_seeds = BTreeMap::new();
    let mut deals: BTreeMap<u64, Deal> = BTreeMap::new();
    let mut groups = Groups::new();
    let mut run_shas = Vec::new();
    for (index, dir) in runs.iter().enumerate() {
        let run: Value = serde_json::from_slice(
            &std::fs::read(dir.join("run.json"))
                .with_context(|| format!("reading {}/run.json", dir.display()))?,
        )?;
        ensure!(
            run["seats"] == 2,
            "self-imitation groups support 1v1 tables only"
        );
        let this = run_identity(&run);
        match &identity {
            None => identity = Some(this),
            Some(first) => ensure!(
                *first == this,
                "{} plays other deals or another policy than the first run",
                dir.display()
            ),
        }
        run_shas.push(crate::files::sha256(&dir.join("run.json"))?);
        let games = std::fs::File::open(dir.join("games.jsonl"))?;
        for line in std::io::BufReader::new(games).lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let game: Value = serde_json::from_str(&line)?;
            let seed = game["policy_seed"]
                .as_u64()
                .context("missing policy_seed")?;
            let owner = *policy_seeds.entry(seed).or_insert(index);
            ensure!(
                owner == index,
                "runs {owner} and {index} share a policy seed"
            );
            let deal_index = game["i"].as_u64().context("missing deal index")?;
            let deal = Deal {
                seed: game["seed"].as_u64().context("missing table seed")?,
                focal_seat: u8::try_from(game["focal_seat"].as_u64().context("missing focal")?)?,
            };
            match deals.entry(deal_index) {
                Entry::Vacant(slot) => {
                    slot.insert(deal);
                }
                Entry::Occupied(known) => ensure!(
                    *known.get() == deal,
                    "deal {deal_index} differs between runs"
                ),
            }
            let record = game["record_sha256"]
                .as_str()
                .context("missing record_sha256")?;
            for seat in net_seats(&game)? {
                let Some(halves) = halves(&game, seat) else {
                    continue;
                };
                groups.entry((deal_index, seat)).or_default().push(Member {
                    run: index,
                    record_sha256: record.to_owned(),
                    seat,
                    halves,
                });
            }
        }
    }
    Ok((groups, run_shas))
}

/// Weights for every net seat that did better than its group.
///
/// # Errors
/// When fewer than two runs are given, the runs disagree on the deals or the
/// policy, a policy seed repeats, or a deal's identity differs between runs.
pub fn group_weights(runs: &[PathBuf]) -> Result<Value> {
    ensure!(
        runs.len() >= 2,
        "a group needs at least two runs of the same deals"
    );
    let (groups, run_shas) = read_groups(runs)?;
    if groups.is_empty() {
        bail!("no finished net games in {runs:?}");
    }
    let mut weights = Vec::new();
    let (mut members, mut mixed, mut usable) = (0_u32, 0_u32, 0_u32);
    let mut run_halves = vec![(0_u32, 0_u32); runs.len()];
    for group in groups.values() {
        let size = u32::try_from(group.len())?;
        members += size;
        for member in group {
            run_halves[member.run].0 += member.halves;
            run_halves[member.run].1 += 1;
        }
        if size < 2 {
            continue;
        }
        usable += 1;
        let total: u32 = group.iter().map(|m| m.halves).sum();
        if group.iter().any(|m| m.halves != group[0].halves) {
            mixed += 1;
        }
        for member in group {
            // Leave-one-out: this outcome against the mean of the others.
            let advantage = (f64::from(member.halves)
                - f64::from(total - member.halves) / f64::from(size - 1))
                / 2.0;
            if advantage > 0.0 {
                weights.push(json!({
                    "run": member.run, "record_sha256": member.record_sha256,
                    "seat": member.seat, "advantage": advantage, "weight": advantage,
                }));
            }
        }
    }
    Ok(json!({
        "version": 1,
        "contract": CONTRACT,
        "runs": runs, "run_sha256": run_shas,
        "summary": {
            "groups": groups.len(), "usable_groups": usable, "mixed_groups": mixed,
            "members": members, "weighted": weights.len(),
            "run_mean_utility": run_halves.iter()
                .map(|&(sum, n)| if n == 0 { 0.0 } else { f64::from(sum) / f64::from(2 * n) })
                .collect::<Vec<_>>(),
        },
        "weights": weights,
    }))
}

/// Writes [`group_weights`] for `runs` to `out`, which must not exist.
///
/// # Errors
/// As [`group_weights`], or when `out` exists or cannot be written.
pub fn write_group_weights(runs: &[PathBuf], out: &Path) -> Result<Value> {
    ensure!(!out.exists(), "{} already exists", out.display());
    let value = group_weights(runs)?;
    crate::files::atomic_json(out, &value)?;
    Ok(value["summary"].clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(dir: &Path, policy_seed: u64, outcomes: &[(u64, u64, u8, Option<u8>)]) {
        std::fs::create_dir_all(dir).unwrap();
        crate::files::atomic_json(
            &dir.join("run.json"),
            &json!({"seats": 2, "decks": ["a", "b"], "planned_deals": 4, "deal_offset": 0,
                "model_identity": {"onnx_sha256": "m"}, "policy_mode": "raw"}),
        )
        .unwrap();
        let lines: Vec<String> = outcomes
            .iter()
            .map(|&(i, seed, focal, winner)| {
                let outcome = match winner {
                    Some(seat) => json!({"kind": "won", "seat": seat}),
                    None => json!({"kind": "draw"}),
                };
                json!({"i": i, "seed": seed, "focal_seat": focal, "policy_seed": policy_seed + i,
                    "record_sha256": format!("r{policy_seed}-{i}"), "complete": true,
                    "capped": false, "outcome": outcome, "actor_roles": null})
                .to_string()
            })
            .collect();
        std::fs::write(dir.join("games.jsonl"), lines.join("\n")).unwrap();
    }

    #[test]
    fn only_games_better_than_their_deal_are_weighted() {
        let root = tempfile::tempdir().unwrap();
        let dirs: Vec<PathBuf> = (0..4).map(|k| root.path().join(format!("r{k}"))).collect();
        // Deal 0: the net (seat 0) wins once in four; deal 1 (seat 1): always loses.
        for (k, dir) in dirs.iter().enumerate() {
            let won = (k == 2).then_some(0);
            run(
                dir,
                1000 * (k as u64 + 1),
                &[(0, 7, 0, won.or(Some(1))), (1, 7, 1, Some(0))],
            );
        }
        let value = group_weights(&dirs).unwrap();
        let weights = value["weights"].as_array().unwrap();
        assert_eq!(weights.len(), 1);
        assert_eq!(weights[0]["run"], 2);
        assert_eq!(weights[0]["seat"], 0);
        assert_eq!(weights[0]["weight"], 1.0);
        assert_eq!(value["summary"]["mixed_groups"], 1);
        assert_eq!(value["summary"]["usable_groups"], 2);
    }

    #[test]
    fn draws_count_half_and_unknown_outcomes_drop_out() {
        let root = tempfile::tempdir().unwrap();
        let dirs: Vec<PathBuf> = (0..2).map(|k| root.path().join(format!("r{k}"))).collect();
        run(&dirs[0], 10, &[(0, 3, 0, None)]);
        run(&dirs[1], 20, &[(0, 3, 0, Some(1))]);
        let value = group_weights(&dirs).unwrap();
        assert_eq!(value["weights"][0]["weight"], 0.5);
    }

    #[test]
    fn runs_must_share_deals_and_policy_but_not_seeds() {
        let root = tempfile::tempdir().unwrap();
        let dirs: Vec<PathBuf> = (0..2).map(|k| root.path().join(format!("r{k}"))).collect();
        run(&dirs[0], 10, &[(0, 3, 0, Some(0))]);
        run(&dirs[1], 10, &[(0, 3, 0, Some(1))]);
        assert!(
            group_weights(&dirs).is_err(),
            "a repeated policy seed is no new sample"
        );
        run(&dirs[1], 20, &[(0, 4, 0, Some(1))]);
        assert!(
            group_weights(&dirs).is_err(),
            "another table seed is another deal"
        );
        assert!(group_weights(&dirs[..1]).is_err());
    }
}
