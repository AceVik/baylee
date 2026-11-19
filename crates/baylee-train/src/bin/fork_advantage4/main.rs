//! What a search's choices are worth in worlds they were not made in, read
//! from the search events of recorded games (`eval4 --search … --record-out`,
//! `games.jsonl`).
//!
//! A searched decision's event holds every candidate's score in every world
//! (`worlds[p][i]`), the same worlds for every candidate. The **fork
//! advantage** of an answer is its score less the policy's own answer's in
//! the same world. A choice made on some worlds and judged on the same ones
//! flatters itself: it has learnt those deals. So, as the house's search
//! was judged (`foresight_probe`):
//!
//! - *on half, judged on half* chooses on every second world (the best mean
//!   advantage, if it clears `--margin`; otherwise the policy's answer) and
//!   judges that choice by its mean advantage on the other worlds, both
//!   ways round. The **lead** is that judged advantage, 0 where the
//!   policy's answer stands, summed over decisions and given per 100;
//! - *in the worlds it was made in* is the same choice judged where it was
//!   made, for the gap between the two.
//!
//! A lead that holds on the other half is a signal a student can learn; one
//! that vanishes there is the search fitting its few worlds. Reported for
//! all decisions, by the kind of the policy's own answer, by the class of
//! the answer a half chose in its place (a planned cast is its spell), by
//! how the rollouts ended, and whether the two halves chose alike. The
//! standard errors are by decision and, since one game's decisions are not
//! independent, by game.
//!
//! The scores are leaf values and results of the search's own worlds, with
//! the searching player in every chair: this measures the teacher, and says
//! nothing about any opponent.

#![allow(clippy::missing_docs_in_private_items)]

use std::collections::BTreeMap;
use std::io::{BufRead as _, BufReader};
use std::path::PathBuf;

use anyhow::Context as _;
use baylee_train::search_labels::{HALVES_MARGIN, Halves, advantage, choose, halves};
use clap::Parser;

#[derive(Parser)]
#[command(about = "Judges a search's choices in worlds they were not made in")]
struct Args {
    /// `games.jsonl` files of searched, recorded runs.
    #[arg(long, required = true, num_args = 1..)]
    games: Vec<PathBuf>,
    /// The mean advantage another answer has to show over the policy's own
    /// on the worlds it is chosen on.
    #[arg(long, default_value_t = HALVES_MARGIN)]
    margin: f64,
    /// Where the report goes, as JSON; it is printed either way.
    #[arg(long)]
    out: Option<PathBuf>,
}

#[derive(serde::Deserialize)]
struct Game {
    #[serde(default)]
    search_events: Vec<Event>,
    /// House answers in the net's seat at the table (a fallback, a stall).
    #[serde(default)]
    fallback_answers: u64,
    /// Answers the table refused.
    #[serde(default)]
    refused_answers: u64,
}

#[derive(serde::Deserialize)]
struct Event {
    candidates: Vec<serde_json::Value>,
    /// The planned cast each candidate begins, where it is one (its taps
    /// and the cast as one candidate).
    #[serde(default)]
    scripts: Vec<serde_json::Value>,
    #[serde(default)]
    worlds: Vec<Vec<f64>>,
    #[serde(default = "accepted")]
    accepted: bool,
    #[serde(default)]
    leaves: Leaves,
}

/// How the event's rollouts ended, counted over every candidate and world.
#[derive(Default, serde::Deserialize)]
struct Leaves {
    /// Rollouts stopped before their horizon: nobody could answer, an
    /// answer was refused, a root cast was taken back, the action cap.
    #[serde(default)]
    stopped: u64,
    /// Of those, rollouts stopped where the next answer would have been the
    /// house's.
    #[serde(default)]
    house_stop: u64,
}

impl Leaves {
    /// The group of an event by how its rollouts ended.
    const fn group(&self) -> &'static str {
        if self.house_stop > 0 {
            "some_rollout_stopped_before_a_house_answer"
        } else if self.stopped > 0 {
            "some_rollout_stopped_otherwise"
        } else {
            "every_rollout_reached_its_end"
        }
    }
}

const fn accepted() -> bool {
    true
}

/// The kind of an answer: its variant's name.
fn kind(action: &serde_json::Value) -> String {
    match action {
        serde_json::Value::String(kind) => kind.clone(),
        serde_json::Value::Object(fields) => fields.keys().next().cloned().unwrap_or_default(),
        _ => String::from("unknown"),
    }
}

/// The class of candidate `at`: a planned cast as `Planned` and its
/// spell's kind, otherwise its answer's kind.
fn class(event: &Event, at: usize) -> String {
    match event.scripts.get(at) {
        Some(script) if !script.is_null() => format!("Planned{}", kind(script)),
        _ => kind(&event.candidates[at]),
    }
}

/// A count as a float, for means.
fn real(n: u64) -> f64 {
    f64::from(u32::try_from(n).unwrap_or(u32::MAX))
}

/// Leads over decisions: their count, how many replaced the policy's
/// answer, the sum and sum of squares of the leads, and per game the
/// decisions and the sum of their leads.
#[derive(Clone, Default)]
struct Leads {
    decisions: u64,
    replaced: u64,
    sum: f64,
    squares: f64,
    /// Per game: (decisions, sum of leads).
    games: BTreeMap<u64, (u64, f64)>,
}

impl Leads {
    fn add(&mut self, game: u64, replaced: bool, lead: f64) {
        self.decisions += 1;
        self.replaced += u64::from(replaced);
        self.sum += lead;
        self.squares += lead * lead;
        let entry = self.games.entry(game).or_default();
        entry.0 += 1;
        entry.1 += lead;
    }

    /// The lead per 100 decisions and its standard error by decision.
    fn per_hundred(&self) -> (f64, f64) {
        if self.decisions == 0 {
            return (0.0, 0.0);
        }
        let n = real(self.decisions);
        let mean = self.sum / n;
        let variance = (self.squares / n - mean * mean).max(0.0);
        (100.0 * mean, 100.0 * (variance / n).sqrt())
    }

    /// The standard error per 100 decisions with each game one cluster:
    /// the ratio estimator's, `G / (G - 1) · Σ (S_g − r·n_g)² / N²`.
    fn per_hundred_se_by_game(&self) -> f64 {
        let clusters = real(self.games.len() as u64);
        if self.decisions == 0 || clusters < 2.0 {
            return 0.0;
        }
        let n = real(self.decisions);
        let ratio = self.sum / n;
        let spread: f64 = self
            .games
            .values()
            .map(|&(decisions, sum)| (sum - ratio * real(decisions)).powi(2))
            .sum();
        100.0 * (clusters / (clusters - 1.0) * spread / (n * n)).sqrt()
    }

    fn report(&self) -> serde_json::Value {
        let (lead, se) = self.per_hundred();
        serde_json::json!({
            "decisions": self.decisions,
            "games": self.games.len(),
            "replaced": self.replaced,
            "replaced_share": if self.decisions == 0 { 0.0 } else {
                real(self.replaced) / real(self.decisions)
            },
            "lead_per_100": lead,
            "lead_per_100_se": se,
            "lead_per_100_se_by_game": self.per_hundred_se_by_game(),
        })
    }
}

/// What the halves chose in the policy's place, by the class of the answer
/// chosen: each way round is half a decision.
#[derive(Clone, Copy, Default)]
struct Replacements {
    /// Ways round that chose this class.
    halves: u64,
    /// The sum of their judged advantages.
    sum: f64,
    /// Of them, judged ahead on the other half.
    held: u64,
}

/// A group of decisions judged both ways.
#[derive(Clone, Default)]
struct Judged {
    /// Chosen on half the worlds, judged on the other half; each decision
    /// counts once, as the mean of the two ways round.
    held_out: Leads,
    /// Chosen on every world and judged on the same ones.
    in_sample: Leads,
    /// Each half's replacement of the policy's answer, by class.
    chosen: BTreeMap<String, Replacements>,
    /// Decisions where a half replaced the policy's answer, and of those
    /// where both halves chose the same candidate, or the same class.
    some_half_replaced: u64,
    halves_agree: u64,
    halves_agree_on_class: u64,
}

impl Judged {
    fn add(&mut self, game: u64, event: &Event, margin: f64) {
        let worlds = &event.worlds;
        let chosen = choose(worlds, margin);
        self.in_sample
            .add(game, chosen != 0, advantage(worlds, chosen));
        let halves = halves(worlds, margin);
        let Halves {
            on_even,
            on_odd,
            judged_even,
            judged_odd,
        } = halves;
        self.held_out
            .add(game, on_even != 0 || on_odd != 0, halves.lead());
        for (at, judged) in [(on_even, judged_even), (on_odd, judged_odd)] {
            if at != 0 {
                let entry = self.chosen.entry(class(event, at)).or_default();
                entry.halves += 1;
                entry.sum += judged;
                entry.held += u64::from(judged > 0.0);
            }
        }
        if on_even != 0 || on_odd != 0 {
            self.some_half_replaced += 1;
            self.halves_agree += u64::from(on_even == on_odd);
            self.halves_agree_on_class += u64::from(class(event, on_even) == class(event, on_odd));
        }
    }

    fn report(&self) -> serde_json::Value {
        let decisions = real(self.held_out.decisions).max(1.0);
        let replaced = real(self.some_half_replaced).max(1.0);
        serde_json::json!({
            "on_half_judged_on_half": self.held_out.report(),
            "in_the_worlds_it_was_made_in": self.in_sample.report(),
            "halves": {
                "some_half_replaced": self.some_half_replaced,
                "both_chose_the_same_answer_share": real(self.halves_agree) / replaced,
                "both_chose_the_same_class_share": real(self.halves_agree_on_class) / replaced,
            },
            "chosen_in_the_policys_place": self
                .chosen
                .iter()
                .map(|(class, r)| {
                    let halves = real(r.halves).max(1.0);
                    (class.clone(), serde_json::json!({
                        "halves": r.halves,
                        "judged_ahead_share": real(r.held) / halves,
                        "mean_judged_advantage": r.sum / halves,
                        "lead_per_100_decisions": 100.0 * r.sum / 2.0 / decisions,
                    }))
                })
                .collect::<BTreeMap<_, _>>(),
        })
    }
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let mut all = Judged::default();
    // The same over the games with no house answer in the net's seat and no
    // refused answer: a game the house helped play is no clean teacher.
    let mut clean = Judged::default();
    let mut clean_games = 0_u64;
    let mut by_kind: BTreeMap<String, Judged> = BTreeMap::new();
    let mut by_leaves: BTreeMap<&'static str, Judged> = BTreeMap::new();
    let (mut games, mut events, mut thin) = (0_u64, 0_u64, 0_u64);
    for path in &args.games {
        let file = std::fs::File::open(path).with_context(|| format!("{}", path.display()))?;
        for line in BufReader::new(file).lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let game: Game = serde_json::from_str(&line)
                .with_context(|| format!("a game of {}", path.display()))?;
            let id = games;
            games += 1;
            let is_clean = game.fallback_answers == 0 && game.refused_answers == 0;
            clean_games += u64::from(is_clean);
            for event in game.search_events {
                events += 1;
                let width = event.candidates.len();
                // Two worlds at least, each scoring every candidate, and an
                // answer the table took.
                if !event.accepted
                    || event.worlds.len() < 2
                    || width < 2
                    || event.worlds.iter().any(|world| world.len() != width)
                {
                    thin += 1;
                    continue;
                }
                all.add(id, &event, args.margin);
                if is_clean {
                    clean.add(id, &event, args.margin);
                }
                by_kind
                    .entry(kind(&event.candidates[0]))
                    .or_default()
                    .add(id, &event, args.margin);
                by_leaves
                    .entry(event.leaves.group())
                    .or_default()
                    .add(id, &event, args.margin);
            }
        }
    }
    let report = serde_json::json!({
        "contract": "fork_advantage_halves_v2",
        "margin": args.margin,
        "games": games,
        "search_events": events,
        "left_out_fewer_than_two_worlds_or_refused": thin,
        "all": all.report(),
        "games_without_house_or_refused_answers": clean_games,
        "in_those_games": clean.report(),
        "by_the_policys_answer": by_kind
            .iter()
            .map(|(kind, judged)| (kind.clone(), judged.report()))
            .collect::<BTreeMap<_, _>>(),
        "by_how_the_rollouts_ended": by_leaves
            .iter()
            .map(|(group, judged)| (*group, judged.report()))
            .collect::<BTreeMap<_, _>>(),
    });
    let text = serde_json::to_string_pretty(&report)?;
    if let Some(out) = &args.out {
        std::fs::write(out, &text).with_context(|| format!("{}", out.display()))?;
    }
    println!("{text}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(candidates: &[serde_json::Value], worlds: Vec<Vec<f64>>) -> Event {
        Event {
            candidates: candidates.to_vec(),
            scripts: Vec::new(),
            worlds,
            accepted: true,
            leaves: Leaves::default(),
        }
    }

    fn pair() -> [serde_json::Value; 2] {
        [
            serde_json::json!("PassPriority"),
            serde_json::json!({"PlayLand": {"card": 3}}),
        ]
    }

    /// A candidate that is ahead in every world keeps its lead on the other
    /// half; one that is ahead only in the worlds it was chosen on shows a
    /// lead in sample and none (here a loss) held out.
    #[test]
    fn a_lead_is_judged_in_worlds_it_was_not_made_in() {
        let mut real = Judged::default();
        real.add(0, &event(&pair(), vec![vec![0.4, 0.6]; 4]), 0.03);
        assert_eq!(real.held_out.replaced, 1);
        assert!((real.held_out.per_hundred().0 - 20.0).abs() < 1e-9);
        assert!((real.in_sample.per_hundred().0 - 20.0).abs() < 1e-9);
        assert_eq!(real.halves_agree, 1);
        assert_eq!(real.chosen["PlayLand"].halves, 2);

        // Ahead in the even worlds by 0.4, behind in the odd ones by 0.3.
        let noise = vec![
            vec![0.3, 0.7],
            vec![0.6, 0.3],
            vec![0.3, 0.7],
            vec![0.6, 0.3],
        ];
        let mut fitted = Judged::default();
        fitted.add(0, &event(&pair(), noise), 0.03);
        assert!(fitted.in_sample.per_hundred().0 > 0.0, "ahead in sample");
        // Chosen on the even worlds it loses 0.3 on the odd ones; on the odd
        // worlds the policy's answer stands.
        assert!((fitted.held_out.per_hundred().0 + 15.0).abs() < 1e-9);
        assert_eq!(fitted.some_half_replaced, 1);
        assert_eq!(fitted.halves_agree, 0, "one half kept the policy's answer");
        assert_eq!(fitted.chosen["PlayLand"].held, 0);
    }

    #[test]
    fn an_answer_is_named_by_its_kind_and_a_planned_cast_by_its_spell() {
        assert_eq!(kind(&serde_json::json!("PassPriority")), "PassPriority");
        assert_eq!(
            kind(&serde_json::json!({"PlayLand": {"card": 3}})),
            "PlayLand"
        );
        let mut planned = event(
            &[
                serde_json::json!({"ActivateManaAbility": {"source": 4}}),
                serde_json::json!({"ActivateManaAbility": {"source": 5}}),
            ],
            vec![vec![0.5, 0.5]; 2],
        );
        planned.scripts = vec![
            serde_json::Value::Null,
            serde_json::json!({"CastSpell": {"card": 9}}),
        ];
        assert_eq!(class(&planned, 0), "ActivateManaAbility");
        assert_eq!(class(&planned, 1), "PlannedCastSpell");
    }

    /// Two games, one ahead and one behind: the error by game sees two
    /// clusters, where the error by decision sees many agreeing decisions.
    #[test]
    fn the_error_by_game_counts_games_not_decisions() {
        let mut leads = Leads::default();
        for _ in 0..50 {
            leads.add(0, true, 0.1);
            leads.add(1, true, -0.1);
        }
        let (mean, by_decision) = leads.per_hundred();
        assert!(mean.abs() < 1e-9);
        assert!(by_decision < 1.01, "{by_decision}");
        let by_game = leads.per_hundred_se_by_game();
        assert!((by_game - 10.0).abs() < 1e-6, "{by_game}");
    }

    #[test]
    fn rollouts_are_grouped_by_how_they_ended() {
        let leaves = |stopped, house_stop| Leaves {
            stopped,
            house_stop,
        };
        assert_eq!(leaves(0, 0).group(), "every_rollout_reached_its_end");
        assert_eq!(leaves(2, 0).group(), "some_rollout_stopped_otherwise");
        assert_eq!(
            leaves(2, 1).group(),
            "some_rollout_stopped_before_a_house_answer"
        );
    }
}
