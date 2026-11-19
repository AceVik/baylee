//! Who made each answer of a run labelled with `eval4 --distill-labels`.
//!
//! Such a run keeps, at every answer of the net's policy, the net's own
//! greedy answer `N` and the tactical layer's answer `R` with the rule
//! groups that changed it (`netplay4::distill`). Where the layer played
//! (`--choice-guard tactical`), an answer equal to `N` is the net's; one that
//! differs is the layer's: a rule group's where a group fired, else the
//! layer's part for the question's head (`priority`, `single`, `subset`,
//! `pair` or `scalar`, as `--choice-guard tactical-<part>` switches them).
//! `deciders` counts them by the card the answer is about and lists the
//! answers about the `--focus` cards; `show` marks the layer's answers.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::Context as _;
use baylee_core::ids::PlayerId;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_train::convert4::distill_labels::DistillLabel4;
use baylee_view::PlayerView;

use crate::game::{Decision, Event, Game};
use crate::losses::{self, Which};
use crate::names::{self, Names};
use crate::show;

/// Who made an answer: `net`, `layer:<part>` or `rule:<groups>`.
pub(super) fn decider(label: &DistillLabel4) -> String {
    if !label.n_ne_r {
        "net".to_owned()
    } else if label.fired.is_empty() {
        format!("layer:{}", label.kind)
    } else {
        format!("rule:{}", label.fired.join("+"))
    }
}

/// What `show` adds to an answer the layer changed: who changed it and what
/// the net would have answered.
pub(super) fn note(label: Option<&DistillLabel4>, names: &Names) -> String {
    match label {
        Some(label) if label.n_ne_r => format!(
            "   ‹{}; net: {}›",
            decider(label),
            label.net_action.as_ref().map_or_else(
                || "?".to_owned(),
                |action| names.annotate(&format!("{action:?}"))
            )
        ),
        _ => String::new(),
    }
}

/// The card an action acts with: the spell cast, the ability's source.
fn acting_card(action: &PlayerAction, names: &Names) -> Option<String> {
    match action {
        PlayerAction::CastSpell { card } => Some(names.name(*card)),
        PlayerAction::ActivateAbility { source, .. } => Some(names.name(*source)),
        PlayerAction::ActivateManaAbility { .. } => Some("(mana)".to_owned()),
        PlayerAction::PlayLand { card, .. } => Some(format!("(land) {}", names.name(*card))),
        _ => None,
    }
}

/// The question's kind with the reason it gives, where it gives one.
fn question(pending: &Pending) -> String {
    let reason = match pending {
        Pending::ChooseCards { prompt, .. } => format!("{prompt:?}"),
        Pending::ChooseNumber { reason, .. } => format!("{reason:?}"),
        _ => return show::kind(pending),
    };
    let reason = reason
        .split([' ', '{', '('])
        .next()
        .unwrap_or("?")
        .to_owned();
    format!("{}/{reason}", show::kind(pending))
}

/// What an answer is about: at priority the card the played answer acts
/// with, else the net's, else `(pass)`; elsewhere the spell being cast, the
/// source being aimed, else the question itself.
fn subject(decision: &Decision<'_>, names: &Names) -> String {
    if matches!(decision.pending, Pending::Priority { .. }) {
        return acting_card(decision.action, names)
            .or_else(|| {
                decision
                    .label
                    .and_then(|label| label.net_action.as_ref())
                    .and_then(|action| acting_card(action, names))
            })
            .unwrap_or_else(|| "(pass)".to_owned());
    }
    let asked = decision.asked;
    asked
        .and_then(|view| view.casting)
        .map(|card| names.name(card))
        .or_else(|| {
            asked
                .and_then(|view| view.targeting.as_ref())
                .map(|targeting| targeting.source.name.to_string())
        })
        .unwrap_or_else(|| question(decision.pending))
}

/// The answers of a set of games, by who made them.
#[derive(Debug, Default)]
struct Tally {
    games: usize,
    labelled: usize,
    /// (subject, head, decider) → answers.
    counts: BTreeMap<(String, String, String), usize>,
    /// The cards to list answer by answer.
    focus: Vec<String>,
    cases: Vec<String>,
    deal: u64,
}

impl Tally {
    fn merge(&mut self, other: Self) {
        self.games += other.games;
        self.labelled += other.labelled;
        for (key, count) in other.counts {
            *self.counts.entry(key).or_default() += count;
        }
        self.cases.extend(other.cases);
    }

    fn decide(&mut self, seat: PlayerId, decision: &Decision<'_>) {
        let Some(label) = decision.label else {
            return;
        };
        self.labelled += 1;
        let names = Names::of(decision.views);
        let about = subject(decision, &names);
        let who = decider(label);
        if self.focus.contains(&about) {
            let table = &decision.views[usize::from(seat.get())];
            self.cases.push(format!(
                "deal {} turn {} {:?}/{:?} {} · {} · {}: {} · {who}{}",
                self.deal,
                table.turn,
                table.phase,
                table.step,
                if table.active == seat {
                    "own turn"
                } else {
                    "their turn"
                },
                position(table, seat),
                question(decision.pending),
                names.annotate(&format!("{:?}", decision.action)),
                note(Some(label), &names)
            ));
        }
        *self
            .counts
            .entry((about, label.kind.clone(), who))
            .or_default() += 1;
    }

    fn print(&self) {
        println!(
            "{} games, {} labelled answers of the net's seat",
            self.games, self.labelled
        );
        // Subjects by how many of their answers the layer changed.
        let mut subjects: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
        for ((about, _, who), count) in &self.counts {
            let entry = subjects.entry(about).or_default();
            entry.0 += count;
            if who != "net" {
                entry.1 += count;
            }
        }
        let mut order: Vec<_> = subjects.into_iter().collect();
        order.sort_by_key(|(about, (_, changed))| (std::cmp::Reverse(*changed), *about));
        println!("\nAnswers by what they are about, the layer's changes first:");
        for (about, (all, changed)) in order.iter().take(60) {
            println!("  {about}: {all} answers, {changed} changed by the layer");
            for ((_, head, who), count) in self.counts.iter().filter(|((a, ..), _)| a == about) {
                println!("      {count:>5}  {head:<8} {who}");
            }
        }
        if !self.focus.is_empty() {
            let mut cases = self.cases.clone();
            cases.sort();
            println!(
                "\nAnswers about {} ({}):",
                self.focus.join(", "),
                cases.len()
            );
            for case in &cases {
                println!("  {case}");
            }
        }
    }
}

/// Both life totals and each side's creatures, as a player would weigh them.
fn position(table: &PlayerView, seat: PlayerId) -> String {
    let lives: Vec<String> = table
        .seats
        .iter()
        .map(|info| {
            let net = if info.player == seat { " (net)" } else { "" };
            format!("{}{net} {}", names::seat(info.player), info.life)
        })
        .collect();
    let creatures = |own: bool| -> Vec<String> {
        table
            .battlefield
            .iter()
            .filter(|object| (object.controller == seat) == own)
            .filter_map(|object| {
                Some(format!(
                    "{} {}/{}",
                    object.name, object.power?, object.toughness?
                ))
            })
            .collect()
    };
    format!(
        "life {} · stack [{}] · own [{}] · theirs [{}]",
        lives.join(", "),
        table
            .stack
            .iter()
            .map(|object| object.name.as_str())
            .collect::<Vec<_>>()
            .join(", "),
        creatures(true).join(", "),
        creatures(false).join(", ")
    )
}

/// The tally of one game.
fn game(run: &Path, deal: u64, focus: &[String]) -> anyhow::Result<Tally> {
    let record = Game::load(run, deal)?;
    let seat = record.seat;
    let mut tally = Tally {
        games: 1,
        deal,
        focus: focus.to_vec(),
        ..Tally::default()
    };
    record.replay(false, |event| {
        if let Event::Decision(decision) = event {
            tally.decide(seat, &decision);
        }
        Ok(())
    })?;
    Ok(tally)
}

/// The deals of `run` itself, in the order its `games.jsonl` keeps them:
/// every game played so far, while the run is still going too.
fn played(run: &Path) -> anyhow::Result<Vec<u64>> {
    let path = run.join("games.jsonl");
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    text.lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            let row: serde_json::Value = serde_json::from_str(line)?;
            row["i"].as_u64().context("a game without its deal")
        })
        .collect()
}

/// Counts who made the answers over the deals of `summary` that `which`
/// selects, or every game of `run` without a summary, replaying them on
/// `threads` threads, and lists the answers about the `focus` cards.
pub(super) fn count(
    run: &Path,
    summary: Option<&Path>,
    which: Which,
    threads: usize,
    focus: &[String],
) -> anyhow::Result<()> {
    let deals: Vec<u64> = match summary {
        Some(summary) => losses::deals(summary, which)?
            .into_iter()
            .map(|deal| deal.number)
            .collect(),
        None => played(run)?,
    };
    let next = AtomicUsize::new(0);
    let total = Mutex::new(Tally {
        focus: focus.to_vec(),
        ..Tally::default()
    });
    let failures = Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for _ in 0..threads.max(1) {
            scope.spawn(|| {
                while let Some(&deal) = deals.get(next.fetch_add(1, Ordering::Relaxed)) {
                    match game(run, deal, focus) {
                        Ok(tally) => total.lock().expect("tally lock").merge(tally),
                        Err(error) => failures
                            .lock()
                            .expect("failures lock")
                            .push(format!("deal {deal}: {error:#}")),
                    }
                }
            });
        }
    });
    let failures = failures.into_inner().expect("failures lock");
    for failure in &failures {
        eprintln!("[gamelog4] {failure}");
    }
    match summary {
        Some(summary) => println!("{which:?} deals of {}", summary.display()),
        None => println!("Every game of {}", run.display()),
    }
    let total = total.into_inner().expect("tally lock");
    anyhow::ensure!(
        total.labelled > 0 || total.games == 0,
        "{} keeps no distillation labels (eval4 --distill-labels)",
        run.display()
    );
    total.print();
    anyhow::ensure!(
        failures.is_empty(),
        "{} games could not be replayed",
        failures.len()
    );
    Ok(())
}
