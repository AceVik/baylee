//! A fair house level answering a chair it does not hold: what a teacher
//! would have played at every question the chair's real player answers.
//!
//! The house-teacher labels (`baylee-train`'s `eval4 --distill-teacher`)
//! ask, at each decision of a seat the trained net plays, what the house
//! would answer there, and keep it beside the net's own answer; the net's
//! answer is the one played. [`Session::teacher_answer`] gives that answer
//! as a house chair with foresight gives its own (`session/foresight.rs`),
//! from the same view, with the same search and the same budget, but for
//! an agent the caller holds ([`Teacher`]) at a chair the table does not
//! play for it.
//!
//! Fair by construction, as a fair chair's search is: the level never
//! peeks ([`Teacher::new`] refuses one that does), its own answer is
//! built from the seat's view and the seat's own deck
//! (`HeuristicAgent::act_with_own_deck`), and a fork always deals the other
//! players' hidden cards from a prior built from that view, never from the
//! real lists, whatever the table lets its own chairs do. Nothing the
//! teacher does changes the game: the table's own searches, their counts
//! and their pending targets are left as they were, and what a search
//! costs is kept in the teacher's own [`ForesightStats`].

use super::Session;
use super::foresight::{Plan, Search, kind, world_seed};
use crate::ForesightStats;
use baylee_ai::HeuristicAgent;
use baylee_ai::intelligence::DeckIntel;
use baylee_core::ids::PlayerId;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_engine::engine::PublicPrior;
use baylee_view::PlayerView;

/// A house level kept by the caller to answer a seat it does not play
/// ([`Session::teacher_answer`]).
#[derive(Debug)]
pub struct Teacher {
    agent: HeuristicAgent,
    /// The target the last search chose with its spell or ability, while
    /// that answer is still the one the seat may have played.
    planned: Option<(PlayerAction, PlayerAction)>,
    /// The target to give where the engine then asks for one: the seat
    /// played the spell the search chose.
    then: Option<PlayerAction>,
    stats: ForesightStats,
}

/// Why a level cannot teach.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TeacherRefusal {
    /// It plays from hidden information (`AIProfile::peeks`).
    Peeks,
}

impl std::fmt::Display for TeacherRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Peeks => f.write_str("a level that peeks never teaches"),
        }
    }
}

impl std::error::Error for TeacherRefusal {}

/// How the teacher came to its answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TeacherHow {
    /// The level's own answer, unsearched: the question is not one a search
    /// takes, the level has no foresight tier, or it had one answer worth
    /// trying.
    Heuristic,
    /// A search ran and kept the level's own answer.
    Kept,
    /// A search ran and chose another answer.
    Replaced,
    /// The engine refused the fork, so the level's own answer stands.
    Refused,
    /// The target a search chose with the spell or ability the seat then
    /// played.
    Planned,
}

impl TeacherHow {
    /// Whether a search ran for this answer.
    #[must_use]
    pub const fn searched(self) -> bool {
        matches!(self, Self::Kept | Self::Replaced)
    }
}

/// What the teacher answered one question with, and how.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TeacherAnswer {
    /// The answer: the search's where one ran and chose, else the level's.
    pub action: PlayerAction,
    /// The level's own answer, before any search.
    pub heuristic: PlayerAction,
    /// How it came to `action`.
    pub how: TeacherHow,
    /// Wall time of the search in microseconds, for measurement only.
    pub micros: Option<u32>,
}

impl Teacher {
    /// A teacher playing `agent`, which the caller has told the sides and
    /// seeded from public data (never the deal's seed). Refuses a level
    /// that peeks.
    ///
    /// # Errors
    ///
    /// [`TeacherRefusal::Peeks`] for a level that plays from hidden
    /// information.
    pub fn new(agent: HeuristicAgent) -> Result<Self, TeacherRefusal> {
        if agent.profile().peeks() {
            return Err(TeacherRefusal::Peeks);
        }
        Ok(Self {
            agent,
            planned: None,
            then: None,
            stats: ForesightStats::default(),
        })
    }

    /// The level this teacher plays.
    #[must_use]
    pub const fn agent(&self) -> &HeuristicAgent {
        &self.agent
    }

    /// What this teacher's searches have cost so far.
    #[must_use]
    pub const fn stats(&self) -> &ForesightStats {
        &self.stats
    }

    /// The seat answered its last question with `action`: the target a
    /// search chose is given next only where this was the spell it chose.
    pub fn played(&mut self, action: &PlayerAction) {
        self.then = self
            .planned
            .take()
            .filter(|(first, _)| first == action)
            .map(|(_, then)| then);
    }
}

impl Session {
    /// What `teacher` answers `player`'s current question with, or `None`
    /// where `player` is asked nothing. The table is not changed: nothing
    /// is played, and its own searches' state and counts stay as they were.
    ///
    /// The answer is the one a house chair at the teacher's level with
    /// foresight gives (`session/foresight.rs`), with the fairness of a
    /// fair chair at a fair table whatever this table lets its own chairs
    /// do: the level's own answer from the seat's view and own deck, and
    /// at a priority decision with nothing begun, an attack or a block, the
    /// answers worth trying played on in forks that deal every hidden card
    /// from the seat's prior. Where the engine refuses the fork, the
    /// level's own answer stands.
    pub fn teacher_answer(
        &mut self,
        player: PlayerId,
        teacher: &mut Teacher,
    ) -> Option<TeacherAnswer> {
        let pending = self.engine.pending_for(player)?.clone();
        let view = self.table().house_view(player, &pending);
        let empty = DeckIntel::new(Vec::new(), Vec::new());
        let deck = self
            .scouting_decks
            .get(usize::from(player.get()))
            .unwrap_or(&empty);
        let agent = teacher.agent.clone();
        let heuristic =
            agent.act_with_own_deck(&view, &pending, &self.engine.decision_context(), deck);
        let own = deck.cards.clone();
        // Nothing the teacher answers is played by it: its memory of the
        // answers it gave must not take the seat's for its own.
        agent.forget_given();
        let plain = |action: PlayerAction, how| TeacherAnswer {
            action,
            heuristic: heuristic.clone(),
            how,
            micros: None,
        };
        if matches!(pending, Pending::ChooseTargets { .. }) {
            if let Some(then) = teacher.then.take()
                && pending.answer_fault(&then).is_none()
            {
                return Some(plain(then, TeacherHow::Planned));
            }
            return Some(plain(heuristic.clone(), TeacherHow::Heuristic));
        }
        let searchable = matches!(
            pending,
            Pending::Priority { .. }
                | Pending::ChooseAttackers { .. }
                | Pending::ChooseBlockers { .. }
        );
        let Some(budget) = agent.profile().foresight().filter(|_| searchable) else {
            return Some(plain(heuristic.clone(), TeacherHow::Heuristic));
        };
        let candidates = agent
            .foresight_candidates(&view, &pending, &heuristic, usize::from(budget.candidates))
            .answers;
        if candidates.len() < 2 {
            return Some(plain(heuristic.clone(), TeacherHow::Heuristic));
        }
        // As a chair's search: a new one forgets the target the last chose.
        teacher.planned = None;
        teacher.then = None;
        teacher.stats.asked += 1;
        let started = std::time::Instant::now();
        let search = Search {
            player,
            prior: Some(
                baylee_ai::foresight::prior::public_prior(&view, &own)
                    .into_iter()
                    .fold(PublicPrior::new(), |prior, (seat, cards)| {
                        prior.with_player(seat, cards)
                    }),
            ),
            decks: self.fork_decks(usize::from(player.get()), true),
            model: agent.simulated(0),
            budget,
            seeds: (0..u64::from(budget.worlds))
                .map(|world| world_seed(agent.seed(), HeuristicAgent::position(&view), world))
                .collect(),
            pending: &pending,
            probe: None,
        };
        let Some((plan, replaced)) = self.teacher_search(&search, teacher, &view, candidates)
        else {
            return Some(plain(heuristic.clone(), TeacherHow::Refused));
        };
        let micros = u32::try_from(started.elapsed().as_micros()).unwrap_or(u32::MAX);
        teacher.stats.micros.push(micros);
        let how = if replaced {
            if let Some(then) = plan.then {
                teacher.planned = Some((plan.first.clone(), then));
            }
            TeacherHow::Replaced
        } else {
            TeacherHow::Kept
        };
        Some(TeacherAnswer {
            micros: Some(micros),
            ..plain(plan.first, how)
        })
    }

    /// `search` run for `teacher` over `candidates` (the level's own
    /// first), counted in the teacher's stats: the plan chosen and whether
    /// it replaced the level's own, or `None` where the engine refused the
    /// fork.
    fn teacher_search(
        &mut self,
        search: &Search<'_>,
        teacher: &mut Teacher,
        view: &PlayerView,
        candidates: Vec<PlayerAction>,
    ) -> Option<(Plan, bool)> {
        let agent = teacher.agent.clone();
        let plans = candidates.into_iter().map(Plan::of).collect();
        // A search counts what it spends into the table's stats; the
        // teacher's are swapped in for it, so the table's own stay its own.
        std::mem::swap(&mut self.foresight_stats, &mut teacher.stats);
        let searched = self.searched(search, &agent, view, plans);
        std::mem::swap(&mut self.foresight_stats, &mut teacher.stats);
        agent.forget_given();
        let Ok((mut plans, chosen)) = searched else {
            teacher.stats.refused += 1;
            return None;
        };
        let kind = kind(search.pending);
        teacher.stats.searched += 1;
        teacher.stats.by_kind[kind].0 += 1;
        if chosen != 0 {
            teacher.stats.replaced += 1;
            teacher.stats.by_kind[kind].1 += 1;
        }
        Some((plans.swap_remove(chosen), chosen != 0))
    }
}
