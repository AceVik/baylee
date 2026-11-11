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
//! all decisions and by the kind of the policy's own answer.
//!
//! The scores are leaf values and results of the search's own worlds, with
//! the searching player in every chair: this measures the teacher, and says
//! nothing about any opponent.

#![allow(clippy::missing_docs_in_private_items)]

use std::collections::BTreeMap;
use std::io::{BufRead as _, BufReader};
use std::path::PathBuf;

use anyhow::Context as _;
use clap::Parser;

#[derive(Parser)]
#[command(about = "Judges a search's choices in worlds they were not made in")]
struct Args {
    /// `games.jsonl` files of searched, recorded runs.
    #[arg(long, required = true, num_args = 1..)]
    games: Vec<PathBuf>,
    /// The mean advantage another answer has to show over the policy's own
    /// on the worlds it is chosen on.
    #[arg(long, default_value_t = 0.03)]
    margin: f64,
    /// Where the report goes, as JSON; it is printed either way.
    #[arg(long)]
    out: Option<PathBuf>,
}

#[derive(serde::Deserialize)]
struct Game {
    #[serde(default)]
    search_events: Vec<Event>,
}

#[derive(serde::Deserialize)]
struct Event {
    candidates: Vec<serde_json::Value>,
    #[serde(default)]
    worlds: Vec<Vec<f64>>,
    #[serde(default = "accepted")]
    accepted: bool,
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

/// A count as a float, for means.
fn real(n: u64) -> f64 {
    f64::from(u32::try_from(n).unwrap_or(u32::MAX))
}

/// The candidate with the best mean advantage over the policy's answer on
/// `worlds`, if it clears `margin`; otherwise the policy's (0). Ties go to
/// the earlier candidate.
fn choose(worlds: &[&Vec<f64>], margin: f64) -> usize {
    let Some(k) = worlds.first().map(|world| world.len()) else {
        return 0;
    };
    let mut chosen = 0;
    let mut best = margin;
    for candidate in 1..k {
        let advantage = advantage(worlds, candidate);
        if advantage > best {
            best = advantage;
            chosen = candidate;
        }
    }
    chosen
}

/// The mean advantage of `candidate` over the policy's answer on `worlds`.
fn advantage(worlds: &[&Vec<f64>], candidate: usize) -> f64 {
    if worlds.is_empty() || candidate == 0 {
        return 0.0;
    }
    let sum: f64 = worlds.iter().map(|world| world[candidate] - world[0]).sum();
    sum / real(worlds.len() as u64)
}

/// Leads over decisions: their count, how many replaced the policy's
/// answer, and the sum and sum of squares of the leads.
#[derive(Clone, Copy, Default)]
struct Leads {
    decisions: u64,
    replaced: u64,
    sum: f64,
    squares: f64,
}

impl Leads {
    fn add(&mut self, replaced: bool, lead: f64) {
        self.decisions += 1;
        self.replaced += u64::from(replaced);
        self.sum += lead;
        self.squares += lead * lead;
    }

    /// The lead per 100 decisions and its standard error.
    fn per_hundred(&self) -> (f64, f64) {
        if self.decisions == 0 {
            return (0.0, 0.0);
        }
        let n = real(self.decisions);
        let mean = self.sum / n;
        let variance = (self.squares / n - mean * mean).max(0.0);
        (100.0 * mean, 100.0 * (variance / n).sqrt())
    }

    fn report(&self) -> serde_json::Value {
        let (lead, se) = self.per_hundred();
        serde_json::json!({
            "decisions": self.decisions,
            "replaced": self.replaced,
            "replaced_share": if self.decisions == 0 { 0.0 } else {
                real(self.replaced) / real(self.decisions)
            },
            "lead_per_100": lead,
            "lead_per_100_se": se,
        })
    }
}

/// A group of decisions judged both ways.
#[derive(Clone, Copy, Default)]
struct Judged {
    /// Chosen on half the worlds, judged on the other half; each decision
    /// counts once, as the mean of the two ways round.
    held_out: Leads,
    /// Chosen on every world and judged on the same ones.
    in_sample: Leads,
}

impl Judged {
    fn add(&mut self, worlds: &[Vec<f64>], margin: f64) {
        let all: Vec<&Vec<f64>> = worlds.iter().collect();
        let chosen = choose(&all, margin);
        self.in_sample.add(chosen != 0, advantage(&all, chosen));
        let even: Vec<&Vec<f64>> = worlds.iter().step_by(2).collect();
        let odd: Vec<&Vec<f64>> = worlds.iter().skip(1).step_by(2).collect();
        let on_even = choose(&even, margin);
        let on_odd = choose(&odd, margin);
        let lead = f64::midpoint(advantage(&odd, on_even), advantage(&even, on_odd));
        self.held_out.add(on_even != 0 || on_odd != 0, lead);
    }

    fn report(&self) -> serde_json::Value {
        serde_json::json!({
            "on_half_judged_on_half": self.held_out.report(),
            "in_the_worlds_it_was_made_in": self.in_sample.report(),
        })
    }
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let mut all = Judged::default();
    let mut by_kind: BTreeMap<String, Judged> = BTreeMap::new();
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
            games += 1;
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
                all.add(&event.worlds, args.margin);
                by_kind
                    .entry(kind(&event.candidates[0]))
                    .or_default()
                    .add(&event.worlds, args.margin);
            }
        }
    }
    let report = serde_json::json!({
        "contract": "fork_advantage_halves_v1",
        "margin": args.margin,
        "games": games,
        "search_events": events,
        "left_out_fewer_than_two_worlds_or_refused": thin,
        "all": all.report(),
        "by_the_policys_answer": by_kind
            .iter()
            .map(|(kind, judged)| (kind.clone(), judged.report()))
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

    /// A candidate that is ahead in every world keeps its lead on the other
    /// half; one that is ahead only in the worlds it was chosen on shows a
    /// lead in sample and none (here a loss) held out.
    #[test]
    fn a_lead_is_judged_in_worlds_it_was_not_made_in() {
        let mut real = Judged::default();
        real.add(&vec![vec![0.4, 0.6]; 4], 0.03);
        assert_eq!(real.held_out.replaced, 1);
        assert!((real.held_out.per_hundred().0 - 20.0).abs() < 1e-9);
        assert!((real.in_sample.per_hundred().0 - 20.0).abs() < 1e-9);

        // Ahead in the even worlds by 0.4, behind in the odd ones by 0.3.
        let noise = vec![
            vec![0.3, 0.7],
            vec![0.6, 0.3],
            vec![0.3, 0.7],
            vec![0.6, 0.3],
        ];
        let mut fitted = Judged::default();
        fitted.add(&noise, 0.03);
        assert!(fitted.in_sample.per_hundred().0 > 0.0, "ahead in sample");
        // Chosen on the even worlds it loses 0.3 on the odd ones; on the odd
        // worlds the policy's answer stands.
        assert!((fitted.held_out.per_hundred().0 + 15.0).abs() < 1e-9);
    }

    #[test]
    fn inside_the_margin_the_policys_answer_stands() {
        let worlds = vec![vec![0.50, 0.52]; 4];
        let all: Vec<&Vec<f64>> = worlds.iter().collect();
        assert_eq!(choose(&all, 0.03), 0);
        assert_eq!(choose(&all, 0.01), 1);
        assert_eq!(advantage(&all, 0), 0.0);
    }

    #[test]
    fn an_answer_is_named_by_its_kind() {
        assert_eq!(kind(&serde_json::json!("PassPriority")), "PassPriority");
        assert_eq!(
            kind(&serde_json::json!({"PlayLand": {"card": 3}})),
            "PlayLand"
        );
    }
}
