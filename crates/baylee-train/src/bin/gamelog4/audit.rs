//! What the evaluated seat does in each decision window, and how often the
//! house at `sharp` would have answered the same in the same position.
//!
//! Windows: the mulligan, priority in the seat's own main phases, priority
//! elsewhere in its own turn (combat, upkeep, end), priority in the
//! opponent's turn, paying a cast (`PlayerView::owed`), attacks, blocks,
//! targets, and every other question. At priority it counts passes, passes
//! while a play was legal (a land, a cast, an activation) and passes while a
//! spell that reads as worth casting was reachable.
//!
//! `sharp`'s answer is a fresh `HeuristicAgent` on the seat's own view of
//! the question, as `eval4 --shadow-teacher sharp` asks it: no history, the
//! seat's hidden information only. A question where the house falls back has
//! no answer of its own and is left out of the agreement. Agreement compares
//! answers with their lists in handle order.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use baylee_ai::HeuristicAgent;
use baylee_core::preset::AIProfile;
use baylee_engine::choice::{LegalActions, Pending, PlayerAction};
use baylee_train::netplay4::reading;
use baylee_view::{Phase, PlayerView};
use serde::Serialize;
use serde_json::json;

use crate::game::{Decision, Event, Game};
use crate::losses::{self, Which};
use crate::show;

/// One window's counts.
#[derive(Clone, Default, Serialize)]
struct Window {
    decisions: u64,
    by_house: u64,
    passes: u64,
    passes_with_a_play: u64,
    passes_with_a_worth_spell: u64,
    sharp_asked: u64,
    sharp_agrees: u64,
    sharp_passes: u64,
    answers: BTreeMap<String, u64>,
}

impl Window {
    fn merge(&mut self, other: &Self) {
        self.decisions += other.decisions;
        self.by_house += other.by_house;
        self.passes += other.passes;
        self.passes_with_a_play += other.passes_with_a_play;
        self.passes_with_a_worth_spell += other.passes_with_a_worth_spell;
        self.sharp_asked += other.sharp_asked;
        self.sharp_agrees += other.sharp_agrees;
        self.sharp_passes += other.sharp_passes;
        for (answer, n) in &other.answers {
            *self.answers.entry(answer.clone()).or_default() += n;
        }
    }
}

/// The window a decision falls in.
fn window(view: &PlayerView, pending: &Pending, seat: baylee_core::ids::PlayerId) -> String {
    match pending {
        Pending::Mulligan { .. } => "mulligan".into(),
        Pending::ChooseAttackers { .. } => "attack".into(),
        Pending::ChooseBlockers { .. } => "block".into(),
        Pending::Priority { .. } if view.owed.is_some() => "payment".into(),
        Pending::Priority { .. } if view.active != seat => "their turn, priority".into(),
        Pending::Priority { .. } if matches!(view.phase, Phase::FirstMain | Phase::SecondMain) => {
            "own main, priority".into()
        }
        Pending::Priority { .. } if view.phase == Phase::Combat => "own combat, priority".into(),
        Pending::Priority { .. } => "own other, priority".into(),
        other => {
            let kind = show::kind(other);
            if kind.starts_with("ChooseTarget") {
                "targets".into()
            } else {
                format!("other: {kind}")
            }
        }
    }
}

/// Whether anything but passing was legal at priority.
fn a_play_was_legal(legal: &LegalActions) -> bool {
    !(legal.lands.is_empty()
        && legal.castable.is_empty()
        && legal.payable.is_empty()
        && legal.abilities.is_empty())
}

/// The answer with its lists in handle order and its variant's name.
fn canonical(action: &PlayerAction) -> PlayerAction {
    let mut action = action.clone();
    match &mut action {
        PlayerAction::DeclareAttackers { attackers } => attackers.sort(),
        PlayerAction::DeclareBlockers { blockers } => blockers.sort(),
        PlayerAction::ChooseObjects { objects } => objects.sort(),
        PlayerAction::ChooseTargets { objects, players }
        | PlayerAction::ChooseTargetBatch {
            objects, players, ..
        } => {
            objects.sort();
            players.sort();
        }
        _ => {}
    }
    action
}

fn variant(action: &PlayerAction) -> String {
    format!("{action:?}")
        .split([' ', '{', '('])
        .next()
        .unwrap_or("?")
        .to_owned()
}

/// Every window of one game.
fn game(run: &Path, deal: u64, seed: u64) -> anyhow::Result<BTreeMap<String, Window>> {
    let record = Game::load(run, deal)?;
    let seat = record.seat;
    let mut windows: BTreeMap<String, Window> = BTreeMap::new();
    let mut asked = 0_u64;
    record.replay(false, |event| {
        let Event::Decision(Decision {
            views,
            asked: context,
            pending,
            action,
            source,
        }) = event
        else {
            return Ok(());
        };
        if source == "Forced" {
            return Ok(());
        }
        let view = context.unwrap_or(&views[usize::from(seat.get())]);
        let entry = windows.entry(window(view, pending, seat)).or_default();
        entry.decisions += 1;
        entry.by_house += u64::from(source == "House");
        *entry.answers.entry(variant(action)).or_default() += 1;
        if let (Pending::Priority { legal, .. }, PlayerAction::PassPriority) = (pending, action) {
            entry.passes += 1;
            entry.passes_with_a_play += u64::from(a_play_was_legal(legal));
            entry.passes_with_a_worth_spell += u64::from(
                reading::reachable_spells(view, legal)
                    .into_iter()
                    .any(|id| reading::worth_casting(view, id)),
            );
        }
        asked += 1;
        let sharp = HeuristicAgent::new(AIProfile::SHARP)
            .with_teams(vec![None; views.len()])
            .with_seed(seed ^ deal.rotate_left(29) ^ asked.rotate_left(17));
        let answer = sharp.act(view, pending);
        if sharp.fallbacks() == 0 {
            entry.sharp_asked += 1;
            entry.sharp_agrees += u64::from(canonical(&answer) == canonical(action));
            entry.sharp_passes += u64::from(matches!(answer, PlayerAction::PassPriority));
        }
        Ok(())
    })?;
    Ok(windows)
}

/// Audits every deal of `summary` in the records of `run`; prints a table
/// and, with `out`, writes the windows as JSON.
pub(super) fn audit(
    run: &Path,
    summary: &Path,
    threads: usize,
    seed: u64,
    out: Option<&Path>,
) -> anyhow::Result<()> {
    let deals = losses::deals(summary, Which::All)?;
    let next = AtomicUsize::new(0);
    let total: Mutex<BTreeMap<String, Window>> = Mutex::new(BTreeMap::new());
    let failures = Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for _ in 0..threads.max(1) {
            scope.spawn(|| {
                while let Some(deal) = deals.get(next.fetch_add(1, Ordering::Relaxed)) {
                    match game(run, deal.number, seed) {
                        Ok(windows) => {
                            let mut total = total.lock().expect("audit lock");
                            for (name, window) in &windows {
                                total.entry(name.clone()).or_default().merge(window);
                            }
                        }
                        Err(error) => failures
                            .lock()
                            .expect("failures lock")
                            .push(format!("deal {}: {error:#}", deal.number)),
                    }
                }
            });
        }
    });
    let failures = failures.into_inner().expect("failures lock");
    anyhow::ensure!(failures.is_empty(), "audit failed: {failures:?}");
    let total = total.into_inner().expect("audit lock");
    println!(
        "| window | decisions | house | passes | pass, play legal | pass, worth spell | sharp agrees | sharp passes |"
    );
    println!("| --- | --- | --- | --- | --- | --- | --- | --- |");
    let share = |n: u64, of: u64| {
        if of == 0 {
            "-".to_owned()
        } else {
            #[allow(clippy::cast_precision_loss, reason = "counts of one run")]
            let percent = 100.0 * n as f64 / of as f64;
            format!("{n} ({percent:.1} %)")
        }
    };
    for (name, w) in &total {
        println!(
            "| {name} | {} | {} | {} | {} | {} | {} | {} |",
            w.decisions,
            w.by_house,
            share(w.passes, w.decisions),
            share(w.passes_with_a_play, w.decisions),
            share(w.passes_with_a_worth_spell, w.decisions),
            share(w.sharp_agrees, w.sharp_asked),
            share(w.sharp_passes, w.sharp_asked),
        );
    }
    if let Some(out) = out {
        std::fs::write(
            out,
            serde_json::to_vec_pretty(&json!({"run": run, "summary": summary,
                "deals": deals.len(), "sharp_seed": seed, "windows": total}))?,
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_core::ids::{Defender, ObjectId, PlayerId};

    /// Two declarations of the same attack in another order are one answer.
    #[test]
    fn answers_compare_with_their_lists_in_handle_order() {
        let o = |n| (ObjectId::new(n, 0), Defender::Player(PlayerId::new(1)));
        let a = PlayerAction::DeclareAttackers {
            attackers: vec![o(2), o(1)],
        };
        let b = PlayerAction::DeclareAttackers {
            attackers: vec![o(1), o(2)],
        };
        assert_eq!(canonical(&a), canonical(&b));
        assert_eq!(variant(&a), "DeclareAttackers");
        assert_ne!(canonical(&a), canonical(&PlayerAction::PassPriority));
    }
}
