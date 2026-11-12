//! `--distill-teacher house:<level>`: house-teacher labels
//! (`house_distillation_v1`). At every decision of a net seat, before the net
//! answers, a fair house level answers the same question from the seat's view
//! (`Session::teacher_answer`: its own answer, and with its foresight tier a
//! search in forks dealt from the seat's public prior); the answer `H` is
//! handed to the net, which labels its own answer with it
//! (`netplay4::distill`), and the net's answer is the one played.
//!
//! The teacher plays the agent a house chair at this table would hold:
//! the level, the sides, and the seed every house chair of a named table is
//! given (`baylee_ai::policy_seed` of the game id and the seat), all public;
//! never the deal's seed. A level that peeks is refused.
//!
//! What each game cost and what the teacher answered is kept beside it in
//! `games.jsonl` (`house_teacher`) and summed for the run's summary: answers
//! by how they were found, the labels by kind and how many of them the net
//! answered otherwise, the teacher's thread CPU time (and wall time), and
//! `H`'s own answers at priority by coarse class (`coarse_class_v1`), split by
//! whether a cost was owed.

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::time::Instant;

use anyhow::{Context as _, bail};
use baylee_core::ids::PlayerId;
use baylee_core::preset::AIProfile;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_gamehost::{PlayerView, Session, Teacher, TeacherHow};
use baylee_train::coarse_class::{COARSE_CLASSES, coarse_class, is_mana_ability_action};
use baylee_train::netplay4::distill::DistillStep4;
use serde_json::{Value, json};

/// How a teacher's answers were found, in [`TeacherHow`]'s order.
const HOW: [&str; 5] = ["heuristic", "kept", "replaced", "refused", "planned"];

/// The coarse classes that start a chain from a free priority: a mana
/// ability (the payment of a spell to come), a spell, an ability.
const CHAIN_START: [&str; 3] = ["tap", "cast", "activate"];

/// The teacher a run names.
#[derive(Clone, Debug)]
pub(crate) struct TeacherSpec {
    spec: String,
    profile: AIProfile,
}

/// The house generation a teacher plays where its spec names none: the one
/// the house-teacher labels were collected at (`AIProfile::FORESIGHT`, the
/// gen-7 expert with foresight). A later build's levels play newer code, so
/// without the pin `house:expert` would quietly name another teacher; a
/// dataset holds one teacher, and a newer one is asked for by name
/// (`house:expert:generation=N`).
pub(crate) const TEACHER_GENERATION: u16 = AIProfile::FORESIGHT;

impl TeacherSpec {
    /// `house:<level>`, the level as `--baseline` names one
    /// (`profile_spec::parse`), at [`TEACHER_GENERATION`] unless its knobs
    /// name a generation.
    ///
    /// # Errors
    /// Another kind of teacher, an unknown level, or a level that peeks.
    pub(crate) fn parse(text: &str) -> anyhow::Result<Self> {
        let level = text.strip_prefix("house:").with_context(|| {
            format!("--distill-teacher {text}: the only teacher is the house, house:<level>")
        })?;
        let names_generation = level
            .split_once(':')
            .is_some_and(|(_, knobs)| knobs.split(',').any(|k| k.starts_with("generation=")));
        let mut profile = super::profile_spec::parse(level)?;
        // A frozen reference from before keeps its own generation.
        if !names_generation && profile.generation > TEACHER_GENERATION {
            profile = profile.at_generation(TEACHER_GENERATION);
        }
        if profile.peeks() {
            bail!("--distill-teacher {text}: a level that peeks never teaches");
        }
        Ok(Self {
            spec: text.to_owned(),
            profile,
        })
    }

    /// What `run.json` says about the teacher.
    pub(crate) fn metadata(&self) -> Value {
        let foresight = self.profile.foresight().map(|budget| {
            json!({"tier": self.profile.foresight, "candidates": budget.candidates,
                "worlds": budget.worlds, "turns": budget.turns, "answers": budget.answers})
        });
        json!({
            "kind": "house",
            "level": self.spec,
            "profile": self.profile,
            "generation": self.profile.generation,
            "foresight": foresight,
            "fair": true,
            "sight": "the seat's own view and deck; every fork deals the hidden cards from the \
                      seat's public prior",
            "seed": "baylee_ai::policy_seed(game id, seat), as a house chair of the table",
            "searched": "priority with nothing begun, attackers, blockers (the level's foresight \
                         tier); the target a search chose, where the seat cast that spell",
        })
    }
}

/// What a teacher answered and cost, over a game or a run.
#[derive(Clone, Debug, Default)]
pub(crate) struct TeacherCounts {
    games: u64,
    /// Answers the teacher gave, by [`HOW`].
    how: [u64; HOW.len()],
    /// Wall and thread CPU time of every answer, in microseconds.
    wall: u64,
    cpu: u64,
    /// Wall and thread CPU time of each searched answer, in microseconds.
    searches: Vec<u32>,
    search_cpu: Vec<u32>,
    /// Worlds the searches played, and the house answers given in them.
    rollouts: u64,
    rollout_answers: u64,
    /// `H` at priority by coarse class: with nothing owed, and paying.
    free: [u64; COARSE_CLASSES.len()],
    paying: [u64; COARSE_CLASSES.len()],
    /// At the same free priorities: the answer the seat played, and `N`
    /// (at a forced step, the one answer there was, which was played).
    played_free: [u64; COARSE_CLASSES.len()],
    net_free: [u64; COARSE_CLASSES.len()],
    /// Free priorities whose `N` is the forced answer, and those with no `N`
    /// (a fallback played after the engine refused the net).
    net_forced: u64,
    net_unknown: u64,
    /// Labels by kind: labelled, and of those the net answered otherwise.
    labels: BTreeMap<String, [u64; 2]>,
    /// Answers the net's menu could not hold, their decisions unlabelled.
    unencodable: u64,
}

impl TeacherCounts {
    fn add(&mut self, other: &Self) {
        self.games += other.games;
        for (sum, n) in self.how.iter_mut().zip(other.how) {
            *sum += n;
        }
        self.wall += other.wall;
        self.cpu += other.cpu;
        self.searches.extend_from_slice(&other.searches);
        self.search_cpu.extend_from_slice(&other.search_cpu);
        self.rollouts += other.rollouts;
        self.rollout_answers += other.rollout_answers;
        for (sum, n) in self.free.iter_mut().zip(other.free) {
            *sum += n;
        }
        for (sum, n) in self.paying.iter_mut().zip(other.paying) {
            *sum += n;
        }
        for (sum, n) in self.played_free.iter_mut().zip(other.played_free) {
            *sum += n;
        }
        for (sum, n) in self.net_free.iter_mut().zip(other.net_free) {
            *sum += n;
        }
        self.net_forced += other.net_forced;
        self.net_unknown += other.net_unknown;
        for (kind, [labelled, differ]) in &other.labels {
            let sum = self.labels.entry(kind.clone()).or_default();
            sum[0] += labelled;
            sum[1] += differ;
        }
        self.unencodable += other.unencodable;
    }

    fn labelled(&self) -> u64 {
        self.labels.values().map(|[labelled, _]| labelled).sum()
    }

    /// The counts as a game's `house_teacher` entry or the run's summary.
    #[allow(clippy::cast_precision_loss, reason = "shares and seconds of counts")]
    pub(crate) fn json(&self) -> Value {
        let sorted = |times: &[u32]| {
            let mut times = times.to_vec();
            times.sort_unstable();
            times
        };
        let (wall, cpu) = (sorted(&self.searches), sorted(&self.search_cpu));
        let at = |times: &[u32], q: f64| {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let index = (times.len().saturating_sub(1) as f64 * q) as usize;
            times
                .get(index)
                .map_or(0.0, |micros| f64::from(*micros) / 1000.0)
        };
        let share = |part: u64, whole: u64| (whole > 0).then(|| part as f64 / whole as f64);
        let labelled = self.labelled();
        let per = |seconds: f64, n: u64| (n > 0).then(|| seconds / n as f64);
        let seconds = self.cpu as f64 / 1e6;
        let search_seconds: f64 = cpu.iter().map(|m| f64::from(*m) / 1e6).sum();
        let classes = |counts: &[u64; COARSE_CLASSES.len()]| {
            COARSE_CLASSES
                .iter()
                .zip(counts)
                .map(|(name, n)| ((*name).to_owned(), json!(n)))
                .collect::<serde_json::Map<_, _>>()
        };
        // One answerer's answers at the free priorities, with its pass and
        // chain-start shares.
        let answered = |counts: &[u64; COARSE_CLASSES.len()]| {
            let answers: u64 = counts.iter().sum();
            let starts: u64 = COARSE_CLASSES
                .iter()
                .zip(counts)
                .filter(|(name, _)| CHAIN_START.contains(name))
                .map(|(_, n)| n)
                .sum();
            json!({"classes": classes(counts), "answers": answers,
                "pass_share": share(counts[0], answers),
                "chain_start_share": share(starts, answers)})
        };
        let house = answered(&self.free);
        let mut net = answered(&self.net_free);
        net["forced"] = json!(self.net_forced);
        net["unknown"] = json!(self.net_unknown);
        json!({
            "games": self.games,
            "answers": self.how.iter().sum::<u64>(),
            "how": HOW.iter().zip(self.how).map(|(name, n)| ((*name).to_owned(), json!(n)))
                .collect::<serde_json::Map<_, _>>(),
            "labelled": labelled,
            "unencodable": self.unencodable,
            "labels": self.labels.iter().map(|(kind, [n, differ])| (kind.clone(), json!({
                "labelled": n, "n_ne_h": differ, "n_ne_h_share": share(*differ, *n)})))
                .collect::<serde_json::Map<_, _>>(),
            "cpu_seconds": seconds,
            "cpu_seconds_per_game": per(seconds, self.games),
            "cpu_seconds_per_label": per(seconds, labelled),
            "wall_seconds": self.wall as f64 / 1e6,
            "search": {
                "searched": cpu.len(),
                "cpu_seconds": search_seconds,
                "cpu_seconds_per_game": per(search_seconds, self.games),
                "cpu_seconds_per_label": per(search_seconds, labelled),
                "cpu_ms_p50": at(&cpu, 0.5), "cpu_ms_p95": at(&cpu, 0.95),
                "cpu_ms_max": at(&cpu, 1.0),
                "wall_ms_p50": at(&wall, 0.5), "wall_ms_p95": at(&wall, 0.95),
                "wall_ms_max": at(&wall, 1.0),
                "rollouts": self.rollouts, "rollout_answers": self.rollout_answers,
            },
            "priority": {
                "free": classes(&self.free),
                "paying": classes(&self.paying),
                "free_answers": house["answers"],
                "pass_share": house["pass_share"],
                "chain_start_share": house["chain_start_share"],
                "chain_start": CHAIN_START,
                "at_free": {
                    "states": "every free priority H answered (Priority, no cost owed), \
                               forced steps included",
                    "house": house,
                    "played": answered(&self.played_free),
                    "net": net,
                },
            },
        })
    }
}

/// Every game's counts, summed for the run's summary.
static TOTAL: Mutex<Option<TeacherCounts>> = Mutex::new(None);

/// The run's teacher summary; `null` where no game was taught.
pub(crate) fn summary() -> Value {
    TOTAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .as_ref()
        .map_or(Value::Null, TeacherCounts::json)
}

/// One game's teachers, one for each net seat it is asked about.
pub(crate) struct GameTeacher<'a> {
    spec: &'a TeacherSpec,
    game_id: String,
    teams: Vec<Option<u8>>,
    seats: BTreeMap<u8, Teacher>,
    /// Whether the question H was last asked at, by seat, was a free
    /// priority: Priority with no cost owed.
    at_free: BTreeMap<u8, bool>,
    counts: TeacherCounts,
}

impl<'a> GameTeacher<'a> {
    /// The teachers of the game `game_id`, at a table of these sides.
    pub(crate) fn new(spec: &'a TeacherSpec, game_id: &str, teams: &[Option<u8>]) -> Self {
        Self {
            spec,
            game_id: game_id.to_owned(),
            teams: teams.to_vec(),
            seats: BTreeMap::new(),
            at_free: BTreeMap::new(),
            counts: TeacherCounts {
                games: 1,
                ..TeacherCounts::default()
            },
        }
    }

    /// `H`, the teacher's answer to `seat`'s question `pending` (the one its
    /// `view` shows), or `None` where the seat is asked nothing.
    ///
    /// # Errors
    /// A level that peeks (refused when the run starts, so never here).
    pub(crate) fn answer(
        &mut self,
        session: &mut Session,
        seat: PlayerId,
        view: &PlayerView,
        pending: &Pending,
    ) -> anyhow::Result<Option<PlayerAction>> {
        let teacher = match self.seats.entry(seat.get()) {
            std::collections::btree_map::Entry::Occupied(entry) => entry.into_mut(),
            std::collections::btree_map::Entry::Vacant(entry) => entry.insert(Teacher::new(
                super::house_seat::agent(&self.game_id, seat, self.spec.profile, &self.teams),
            )?),
        };
        let free = matches!(pending, Pending::Priority { .. }) && view.owed.is_none();
        self.at_free.insert(seat.get(), free);
        let (started, cpu_started) = (Instant::now(), thread_cpu_micros());
        let Some(answer) = session.teacher_answer(seat, teacher) else {
            return Ok(None);
        };
        let cpu = thread_cpu_micros().saturating_sub(cpu_started);
        self.counts.wall += u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX);
        self.counts.cpu += cpu;
        self.counts.how[how_index(answer.how)] += 1;
        if let Some(micros) = answer.micros {
            self.counts.searches.push(micros);
            self.counts
                .search_cpu
                .push(u32::try_from(cpu).unwrap_or(u32::MAX));
        }
        if matches!(pending, Pending::Priority { .. }) {
            let tap = is_mana_ability_action(view, &answer.action);
            let class = usize::from(coarse_class(Some(&answer.action), tap));
            let counts = if view.owed.is_some() {
                &mut self.counts.paying
            } else {
                &mut self.counts.free
            };
            if let Some(n) = counts.get_mut(class) {
                *n += 1;
            }
        }
        Ok(Some(answer.action))
    }

    /// What `seat` played at the question `H` was last asked (its `view`),
    /// and `N` there (`net`; with `forced`, the net had one answer and so
    /// none of its own): counted beside `H` where that question was a free
    /// priority. Before [`Self::played`], which forgets the question.
    pub(crate) fn observed(
        &mut self,
        seat: PlayerId,
        view: &PlayerView,
        played: &PlayerAction,
        net: Option<&PlayerAction>,
        forced: bool,
    ) {
        if self.at_free.get(&seat.get()) != Some(&true) {
            return;
        }
        let class = |action: &PlayerAction| {
            usize::from(coarse_class(
                Some(action),
                is_mana_ability_action(view, action),
            ))
        };
        let bump = |counts: &mut [u64; COARSE_CLASSES.len()], class: usize| {
            if let Some(n) = counts.get_mut(class) {
                *n += 1;
            }
        };
        bump(&mut self.counts.played_free, class(played));
        match (net, forced) {
            (Some(net), _) => bump(&mut self.counts.net_free, class(net)),
            (None, true) => {
                bump(&mut self.counts.net_free, class(played));
                self.counts.net_forced += 1;
            }
            (None, false) => self.counts.net_unknown += 1,
        }
    }

    /// `seat` played `action`: the target a search chose stays planned
    /// only where it was the spell it chose.
    pub(crate) fn played(&mut self, seat: PlayerId, action: &PlayerAction) {
        self.at_free.remove(&seat.get());
        if let Some(teacher) = self.seats.get_mut(&seat.get()) {
            teacher.played(action);
        }
    }

    /// The game's counts, with its labels (`steps`) and the answers the
    /// net's menus could not hold, added to the run's total.
    pub(crate) fn finish(mut self, steps: Option<&[DistillStep4]>, unencodable: u64) -> Value {
        for label in steps
            .unwrap_or_default()
            .iter()
            .filter_map(|step| step.label.as_ref())
        {
            let counts = self.counts.labels.entry(label.kind.clone()).or_default();
            counts[0] += 1;
            counts[1] += u64::from(label.n_ne_r);
        }
        self.counts.unencodable = unencodable;
        for teacher in self.seats.values() {
            self.counts.rollouts += teacher.stats().rollouts;
            self.counts.rollout_answers += teacher.stats().answers;
        }
        TOTAL
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get_or_insert_with(TeacherCounts::default)
            .add(&self.counts);
        self.counts.json()
    }
}

/// The CPU time this thread has used, in microseconds: what a teacher's
/// answer cost wherever other work shares the machine.
fn thread_cpu_micros() -> u64 {
    let now = rustix::time::clock_gettime(rustix::time::ClockId::ThreadCPUTime);
    u64::try_from(now.tv_sec).unwrap_or(0) * 1_000_000
        + u64::try_from(now.tv_nsec / 1000).unwrap_or(0)
}

const fn how_index(how: TeacherHow) -> usize {
    match how {
        TeacherHow::Heuristic => 0,
        TeacherHow::Kept => 1,
        TeacherHow::Replaced => 2,
        TeacherHow::Refused => 3,
        TeacherHow::Planned => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::{TeacherCounts, TeacherSpec};

    #[test]
    fn only_a_fair_house_level_teaches() {
        let expert = TeacherSpec::parse("house:expert").unwrap();
        assert_eq!(expert.metadata()["generation"], 7);
        assert_eq!(expert.metadata()["foresight"]["worlds"], 6);
        assert_eq!(expert.metadata()["fair"], true);
        assert!(TeacherSpec::parse("house:sharp-2026-11").is_ok());
        // The pin: a later build's expert teaches as the gen-7 one did unless
        // a generation is named; a frozen reference keeps its own.
        assert_eq!(
            TeacherSpec::parse("house:sharp-2026-11")
                .unwrap()
                .metadata()["generation"],
            0
        );
        if baylee_core::preset::AIProfile::GENERATION > super::TEACHER_GENERATION {
            let named = format!(
                "house:expert:generation={}",
                baylee_core::preset::AIProfile::GENERATION
            );
            assert_eq!(
                TeacherSpec::parse(&named).unwrap().metadata()["generation"],
                baylee_core::preset::AIProfile::GENERATION
            );
        }
        for refused in ["house:seer", "expert", "llm:carbon", "house:nobody"] {
            assert!(TeacherSpec::parse(refused).is_err(), "{refused}");
        }
    }

    #[test]
    fn the_counts_name_the_pass_share_and_the_chain_starts_at_a_free_priority() {
        let mut counts = TeacherCounts {
            games: 1,
            cpu: 2_000_000,
            free: [6, 2, 1, 1, 0, 0],
            paying: [0, 3, 0, 0, 0, 0],
            played_free: [8, 1, 1, 0, 0, 0],
            net_free: [9, 0, 1, 0, 0, 0],
            net_forced: 3,
            ..TeacherCounts::default()
        };
        counts.labels.insert("priority".into(), [10, 4]);
        let mut total = TeacherCounts::default();
        total.add(&counts);
        total.add(&counts);
        let json = total.json();
        assert_eq!(json["games"], 2);
        assert_eq!(json["labelled"], 20);
        assert_eq!(json["labels"]["priority"]["n_ne_h"], 8);
        assert_eq!(json["priority"]["free_answers"], 20);
        assert_eq!(json["priority"]["pass_share"], 0.6);
        assert_eq!(json["priority"]["chain_start_share"], 0.4);
        assert_eq!(json["cpu_seconds_per_label"], 0.2);
        // The played answer and N at the same free priorities, beside H.
        let at_free = &json["priority"]["at_free"];
        assert_eq!(at_free["house"]["pass_share"], 0.6);
        assert_eq!(at_free["played"]["answers"], 20);
        assert_eq!(at_free["played"]["pass_share"], 0.8);
        assert_eq!(at_free["played"]["chain_start_share"], 0.2);
        assert_eq!(at_free["net"]["chain_start_share"], 0.1);
        assert_eq!(at_free["net"]["forced"], 6);
    }
}
