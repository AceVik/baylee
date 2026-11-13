use anyhow::{Context, Result, ensure};
use baylee_ai::{HeuristicAgent, intelligence::DeckIntel};
use baylee_core::{
    ids::PlayerId,
    preset::{AIProfile, GamePreset},
};
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_gamehost::Session;
use baylee_view::PlayerView;

use super::Limits;

#[derive(Clone, Copy)]
pub(super) enum Solver {
    Tactic(u8),
    House { profile: AIProfile, seed: u64 },
}

pub(super) struct Run {
    pub session: Session,
    pub record: Vec<u8>,
    pub trace: super::semantic::Trace,
    actions: Vec<PlayerAction>,
}

fn session(preset: &GamePreset) -> Result<Session> {
    let mut session =
        Session::new_recorded(preset, baylee_build::short()).context("challenge session")?;
    session.describe(uuid::Uuid::now_v7().to_string(), Vec::new());
    session.discard_frames();
    Ok(session)
}

pub(super) fn run(preset: &GamePreset, solver: Solver, limits: Limits) -> Result<Option<Run>> {
    let mut session = session(preset)?;
    let (profile, seed) = match solver {
        Solver::Tactic(_) => (AIProfile::EXPERT, 71),
        Solver::House { profile, seed } => (profile, seed),
    };
    let agent = HeuristicAgent::new(profile).with_seed(seed);
    let deck = DeckIntel::new(
        preset.seats[0].deck.iter().map(|card| card.card).collect(),
        Vec::new(),
    );
    let mut tactic = Tactic::default();
    let mut actions = Vec::new();
    let mut record = Vec::new();
    let mut trace = super::semantic::Trace::default();
    for _ in 0..limits.answers {
        session.pump_at_most(1);
        trace.observe(&session);
        record.extend(session.take_record());
        if let Pending::GameOver(result) = session.pending() {
            return Ok(
                (session.winning_seats(*result) == [PlayerId::new(0)]).then_some(Run {
                    session,
                    record,
                    trace,
                    actions,
                }),
            );
        }
        if session.state().turn.number > limits.turns {
            return Ok(None);
        }
        if session.needs_pump() {
            continue;
        }
        let Some((pending, view)) = session.view_for(PlayerId::new(0)) else {
            continue;
        };
        let note = session.question_note(PlayerId::new(0));
        let action = match solver {
            Solver::Tactic(route) => tactic.answer(route, &view, &pending),
            Solver::House { .. } => None,
        }
        .unwrap_or_else(|| {
            baylee_ai::noted::answer_as_seated(&agent, view, &pending, note.as_ref(), Some(&deck))
        });
        ensure!(
            agent.fallbacks() == 0,
            "solver needed an unsupported-choice fallback"
        );
        session
            .act_slice(PlayerId::new(0), action.clone(), 0)
            .map_err(|error| anyhow::anyhow!("certification answer refused: {error}"))?;
        actions.push(action);
        trace.observe(&session);
        record.extend(session.take_record());
    }
    Ok(None)
}

pub(super) fn verify_fresh(preset: &GamePreset, run: &Run) -> Result<()> {
    let mut fresh = session(preset)?;
    let mut record = Vec::new();
    for action in &run.actions {
        fresh.pump_at_most(128);
        record.extend(fresh.take_record());
        fresh
            .act_slice(PlayerId::new(0), action.clone(), 0)
            .map_err(anyhow::Error::msg)?;
        record.extend(fresh.take_record());
    }
    fresh.pump_at_most(128);
    record.extend(fresh.take_record());
    ensure!(
        matches!(fresh.pending(), Pending::GameOver(_)),
        "fresh run did not finish"
    );
    ensure!(
        fresh.snapshot_hash() == run.session.snapshot_hash() && record == run.record,
        "fresh opponent execution diverged"
    );
    let resumed = Session::resume_recorded(&record)
        .map_err(|error| anyhow::anyhow!("policy reconstruction failed: {error:?}"))?;
    ensure!(
        resumed.snapshot_hash() == fresh.snapshot_hash(),
        "resumed verification diverged"
    );
    Ok(())
}

#[derive(Default)]
struct Tactic {
    next: usize,
    target: u8,
}

impl Tactic {
    fn answer(&mut self, route: u8, view: &PlayerView, pending: &Pending) -> Option<PlayerAction> {
        let plan = match route {
            0 => [("Lightning Bolt", 0), ("Shock", 0)],
            1 => [("Giant Growth", 1), ("Fling", 0)],
            _ => [("Lightning Bolt", 2), ("Giant Growth", 1)],
        };
        match pending {
            Pending::Mulligan { .. } => Some(PlayerAction::MulliganKeep),
            Pending::Priority { legal, .. } if view.owed.is_none() => {
                if view.active == view.seat
                    && view.stack.is_empty()
                    && let Some((name, target)) = plan.get(self.next)
                    && let Some(card) = view.hand.iter().find(|card| {
                        card.name == *name
                            && legal
                                .castable
                                .iter()
                                .chain(&legal.payable)
                                .any(|id| *id == card.id)
                    })
                {
                    self.next += 1;
                    self.target = *target;
                    return Some(PlayerAction::CastSpell { card: card.id });
                }
                Some(PlayerAction::PassPriority)
            }
            Pending::ChooseTargets {
                options,
                player_options,
                ..
            } => {
                if self.target == 0 && player_options.contains(&PlayerId::new(1)) {
                    return Some(PlayerAction::ChooseTargets {
                        objects: Vec::new(),
                        players: vec![PlayerId::new(1)],
                    });
                }
                let owner = PlayerId::new(u8::from(self.target == 2));
                let object = options.iter().find(|id| {
                    view.object(**id).is_some_and(|object| {
                        object.controller == owner && object.name.as_ref() == "Grizzly Bears"
                    })
                })?;
                Some(PlayerAction::ChooseObjects {
                    objects: vec![*object],
                })
            }
            Pending::ChooseCards {
                options,
                min: 1,
                max: 1,
                ..
            } if route == 1 => {
                let object = options
                    .iter()
                    .filter_map(|id| view.object(*id))
                    .max_by_key(|object| object.power)?;
                Some(PlayerAction::ChooseObjects {
                    objects: vec![object.id],
                })
            }
            Pending::ChooseAttackers { .. } if route != 2 => Some(PlayerAction::DeclareAttackers {
                attackers: Vec::new(),
            }),
            _ => None,
        }
    }
}
