//! `--distill`: what the tactical layer's answers `R` are worth beside the
//! net's own `N`, read from the probes a measuring search kept with the
//! distillation labels (`eval4 --search-rival distill`, `distill_probe_v1`).
//!
//! A probe holds `N`'s and `R`'s scores in the same fair worlds; `R`'s fork
//! advantage there is its score less `N`'s. Per probe the mean over the
//! worlds and its standard error give a verdict: `R` ahead (mean − se > 0),
//! `N` ahead (mean + se < 0), or neither. Reported for all probes, in the
//! games no house answer or refusal touched, by the decision's subtype, by
//! each rule group that fired (`none` where the layer's own reading changed
//! the answer), by the kinds of `R` and `N`, and by how the rollouts ended.
//! Per cell: the mean advantage and the share `R` ahead less the share `N`
//! ahead, each per 100 probes with its standard error by game (one game's
//! decisions are not independent), and how often the two halves of the
//! worlds agree on the sign.

use std::collections::BTreeMap;
use std::io::{BufRead as _, BufReader};

use anyhow::Context as _;
use baylee_train::convert4::distill_labels::{DistillLabel4, DistillProbe4, DistillStep4};
use serde_json::{Value, json};

use super::leads::{Leads, real};
use super::{Args, kind};

/// A game line's part this report reads.
#[derive(serde::Deserialize)]
struct Game {
    #[serde(default)]
    distill_sidecar: Vec<DistillStep4>,
    /// House answers in the net's seat at the table (a fallback, a stall).
    #[serde(default)]
    fallback_answers: u64,
    /// Answers the table refused.
    #[serde(default)]
    refused_answers: u64,
}

/// One probe's verdict.
struct Verdict {
    mean: f64,
    r_ahead: bool,
    n_ahead: bool,
    /// Both halves have a world and agree on the sign of the advantage.
    halves: Option<bool>,
    /// A rollout stopped short: nobody could answer, an answer was refused
    /// or taken back, or a house answer was next.
    stopped: bool,
}

impl Verdict {
    fn of(probe: &DistillProbe4) -> Option<Self> {
        let (mean, se) = probe.advantage()?;
        let r_ahead = se.is_some_and(|se| mean - se > 0.0);
        let n_ahead = se.is_some_and(|se| mean + se < 0.0);
        let halves = match probe.halves() {
            [Some(even), Some(odd)] if even != 0.0 && odd != 0.0 => {
                Some(even.signum() == odd.signum())
            }
            [Some(_), Some(_)] => Some(false),
            _ => None,
        };
        let stopped = probe
            .ends
            .iter()
            .flatten()
            .any(|end| matches!(end.as_str(), "stopped" | "house_stop" | "taken_back"));
        Some(Self {
            mean,
            r_ahead,
            n_ahead,
            halves,
            stopped,
        })
    }
}

/// What the probes of one cell say.
#[derive(Clone, Default)]
struct Cell {
    /// The mean advantage of `R` per probe (`replaced`: `R` ahead).
    advantage: Leads,
    /// `+1` where `R` is ahead, `−1` where `N` is, `0` otherwise
    /// (`replaced`: `N` ahead).
    verdict: Leads,
    halves_agree: u64,
    halves_both: u64,
    stopped: u64,
}

impl Cell {
    fn add(&mut self, game: u64, verdict: &Verdict) {
        self.advantage.add(game, verdict.r_ahead, verdict.mean);
        let signed = if verdict.r_ahead {
            1.0
        } else if verdict.n_ahead {
            -1.0
        } else {
            0.0
        };
        self.verdict.add(game, verdict.n_ahead, signed);
        if let Some(agree) = verdict.halves {
            self.halves_both += 1;
            self.halves_agree += u64::from(agree);
        }
        self.stopped += u64::from(verdict.stopped);
    }

    fn report(&self) -> Value {
        let probes = self.advantage.decisions;
        let share = |n: u64| {
            if probes == 0 {
                0.0
            } else {
                real(n) / real(probes)
            }
        };
        let z = |mean: f64, se: f64| if se > 0.0 { mean / se } else { 0.0 };
        let (advantage, advantage_se) = self.advantage.per_hundred();
        let advantage_by_game = self.advantage.per_hundred_se_by_game();
        let (verdict, _) = self.verdict.per_hundred();
        let verdict_by_game = self.verdict.per_hundred_se_by_game();
        json!({
            "probes": probes,
            "games": self.advantage.games.len(),
            "advantage_per_100": advantage,
            "advantage_per_100_se": advantage_se,
            "advantage_per_100_se_by_game": advantage_by_game,
            "advantage_z_by_game": z(advantage, advantage_by_game),
            "r_ahead": self.advantage.replaced,
            "n_ahead": self.verdict.replaced,
            "r_ahead_share": share(self.advantage.replaced),
            "n_ahead_share": share(self.verdict.replaced),
            "r_less_n_ahead_per_100": verdict,
            "r_less_n_ahead_per_100_se_by_game": verdict_by_game,
            "r_less_n_ahead_z_by_game": z(verdict, verdict_by_game),
            "halves_agree_share": if self.halves_both == 0 { 0.0 } else {
                real(self.halves_agree) / real(self.halves_both)
            },
            "with_a_rollout_stopped_short": self.stopped,
        })
    }
}

fn cells(map: &BTreeMap<String, Cell>) -> Value {
    Value::Object(
        map.iter()
            .map(|(key, cell)| (key.clone(), cell.report()))
            .collect(),
    )
}

/// The kinds of `R` and `N`: `R CastSpell / N PassPriority`.
fn answers(label: &DistillLabel4) -> String {
    let named = |action: Option<&baylee_engine::choice::PlayerAction>| {
        action
            .and_then(|a| serde_json::to_value(a).ok())
            .map_or_else(|| "none".to_owned(), |a| kind(&a))
    };
    format!(
        "R {} / N {}",
        named(label.rules_action.as_ref()),
        named(label.net_action.as_ref())
    )
}

/// What every cell of the probes in `args.games` says.
#[derive(Default)]
struct Report {
    games: u64,
    games_with_probes: u64,
    clean_games: u64,
    labels: u64,
    n_ne_r: u64,
    thin: u64,
    all: Cell,
    clean: Cell,
    by_subtype: BTreeMap<String, Cell>,
    by_group: BTreeMap<String, Cell>,
    by_answers: BTreeMap<String, Cell>,
    by_ends: BTreeMap<String, Cell>,
}

impl Report {
    fn game(&mut self, game: &Game) {
        let id = self.games;
        self.games += 1;
        let clean = game.fallback_answers == 0 && game.refused_answers == 0;
        self.clean_games += u64::from(clean);
        let mut probed = false;
        for label in game.distill_sidecar.iter().filter_map(|s| s.label.as_ref()) {
            self.labels += 1;
            self.n_ne_r += u64::from(label.n_ne_r);
            let Some(probe) = &label.probe else {
                continue;
            };
            let Some(verdict) = Verdict::of(probe).filter(|_| probe.worlds.len() >= 2) else {
                self.thin += 1;
                continue;
            };
            probed = true;
            self.all.add(id, &verdict);
            if clean {
                self.clean.add(id, &verdict);
            }
            let subtype = label.subtype.map_or("none", |s| s.name());
            self.by_subtype
                .entry(subtype.to_owned())
                .or_default()
                .add(id, &verdict);
            if label.fired.is_empty() {
                self.by_group
                    .entry("none".to_owned())
                    .or_default()
                    .add(id, &verdict);
            }
            for group in &label.fired {
                self.by_group
                    .entry(group.clone())
                    .or_default()
                    .add(id, &verdict);
            }
            self.by_answers
                .entry(answers(label))
                .or_default()
                .add(id, &verdict);
            let ends = if verdict.stopped {
                "some_rollout_stopped_short"
            } else {
                "every_rollout_reached_its_end"
            };
            self.by_ends
                .entry(ends.to_owned())
                .or_default()
                .add(id, &verdict);
        }
        self.games_with_probes += u64::from(probed);
    }

    fn json(&self) -> Value {
        json!({
            "contract": "distill_probe_report_v1",
            "verdict": "R ahead: mean - se > 0; N ahead: mean + se < 0; se over the probe's worlds",
            "games": self.games,
            "games_with_probes": self.games_with_probes,
            "labels": self.labels,
            "labels_n_ne_r": self.n_ne_r,
            "left_out_fewer_than_two_worlds": self.thin,
            "all": self.all.report(),
            "games_without_house_or_refused_answers": self.clean_games,
            "in_those_games": self.clean.report(),
            "by_subtype": cells(&self.by_subtype),
            "by_group": cells(&self.by_group),
            "by_answers": cells(&self.by_answers),
            "by_how_the_rollouts_ended": cells(&self.by_ends),
        })
    }
}

/// The report over the labels of `args.games`.
///
/// # Errors
/// A file that cannot be read, or a line that is not a game.
pub(super) fn report(args: &Args) -> anyhow::Result<Value> {
    let mut report = Report::default();
    for path in &args.games {
        let file = std::fs::File::open(path).with_context(|| format!("{}", path.display()))?;
        for line in BufReader::new(file).lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let game: Game = serde_json::from_str(&line)
                .with_context(|| format!("a game of {}", path.display()))?;
            report.game(&game);
        }
    }
    Ok(report.json())
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_train::convert4::distill_labels::DistillSubtype4;
    use baylee_train::features4::TeacherTarget4;

    fn label(fired: &[&str], worlds: Option<Vec<[f32; 2]>>) -> DistillLabel4 {
        DistillLabel4 {
            kind: "priority".into(),
            pending_kind: 0,
            menu_sha256: String::new(),
            rules: TeacherTarget4::default(),
            rules_action: Some(baylee_engine::choice::PlayerAction::PassPriority),
            net: TeacherTarget4::default(),
            net_action: Some(baylee_engine::choice::PlayerAction::CancelCast),
            executed: TeacherTarget4::default(),
            fired: fired.iter().map(|g| (*g).to_owned()).collect(),
            n_ne_r: true,
            subtype: Some(DistillSubtype4::Pass),
            probe: worlds.map(|worlds| DistillProbe4 {
                rival_index: 0,
                own_index: 1,
                ends: worlds
                    .iter()
                    .map(|_| ["boundary".to_owned(), "boundary".to_owned()])
                    .collect(),
                worlds,
                rollouts: 4,
                house_answers: 0,
            }),
        }
    }

    fn game(labels: Vec<DistillLabel4>) -> Game {
        Game {
            distill_sidecar: labels
                .into_iter()
                .enumerate()
                .map(|(i, label)| DistillStep4 {
                    seat: 0,
                    decision_seq: i as u64,
                    record_byte_offset: i,
                    label: Some(label),
                })
                .collect(),
            fallback_answers: 0,
            refused_answers: 0,
        }
    }

    #[test]
    fn probes_are_judged_by_their_worlds_and_pooled_by_game() {
        let ahead = vec![[0.4, 0.6], [0.5, 0.6], [0.4, 0.5], [0.5, 0.7]];
        let behind = vec![[0.6, 0.4], [0.7, 0.6], [0.6, 0.5], [0.6, 0.4]];
        let even = vec![[0.5, 0.6], [0.5, 0.4], [0.5, 0.6], [0.5, 0.4]];
        let mut report = Report::default();
        report.game(&game(vec![
            label(&["react"], Some(ahead.clone())),
            label(&[], Some(behind)),
            label(&[], None),
            label(&["react"], Some(vec![[0.5, 0.9]])),
        ]));
        report.game(&game(vec![
            label(&["react", "spend"], Some(ahead)),
            label(&[], Some(even)),
        ]));
        let json = report.json();
        assert_eq!(json["labels"], 6);
        assert_eq!(json["left_out_fewer_than_two_worlds"], 1, "one world");
        assert_eq!(json["games_with_probes"], 2);
        let all = &json["all"];
        assert_eq!(all["probes"], 4);
        assert_eq!(
            (all["r_ahead"].as_u64(), all["n_ahead"].as_u64()),
            (Some(2), Some(1))
        );
        assert!((all["r_less_n_ahead_per_100"].as_f64().unwrap() - 25.0).abs() < 1e-9);
        assert_eq!(json["by_group"]["react"]["probes"], 2);
        assert_eq!(json["by_group"]["spend"]["probes"], 1);
        assert_eq!(json["by_group"]["none"]["probes"], 2);
        assert_eq!(json["by_subtype"]["pass"]["probes"], 4);
        assert_eq!(
            json["by_answers"]["R PassPriority / N CancelCast"]["probes"],
            4
        );
        // The even-odd probe disagrees between its halves; the others agree.
        assert!((all["halves_agree_share"].as_f64().unwrap() - 0.75).abs() < 1e-9);
        assert_eq!(all["games"], 2);
    }
}
