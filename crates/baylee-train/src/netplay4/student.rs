//! The search student (`search_advantage_head_v1`): a small head on the
//! net's own embeddings that learnt, from the fair search's fork advantages
//! (`search_advantage_v1`), where an answer beats the seat's own by a clear
//! margin (`docs/trained-ai.md` §"The search student").
//!
//! At a macro root (the search's roots: priority at the start of a chain
//! where the seat acts, in its own main phase, or facing another player's
//! object) the seat answers as always. Then the net reads the root once more
//! with the payable menu ([`PayableMenu4::extend`]: each spell the planner
//! pays for is one item), from the same memory, which the pass leaves as it
//! was. The head scores the items the search's macro candidates stand for
//! ([`crate::search_labels::macro_items`]) and the seat's own answer's
//! item; the best of them is played in place of the seat's answer only when
//! its score is above the seat's by more than the head's margin. A chosen
//! planned cast is then paid by the planner's taps, answer by answer, as
//! `--menu-payable` pays its own.
//!
//! The seat's own menu is never changed, so where the head keeps the seat's
//! answer the seat plays exactly as without it. Nothing here reads the
//! session: the menu, the plan and the rows come from the seat's view.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use anyhow::Context as _;
use baylee_ai::HeuristicAgent;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_view::PlayerView;
use ort::session::Session;
use ort::session::builder::GraphOptimizationLevel;
use ort::value::Tensor;
use serde_json::Value;

use super::menu::PayableMenu4;
use super::search::{StallMemory4, macro_candidates, macro_root_skip};
use super::{
    Answer4, ModelIdentity, NetPlayer4, RECURRENT_SLOTS, RootDecision4, StepEmbeddings4, ort_error,
};
use crate::coarse_class::{COARSE_CLASSES, NO_CLASS, coarse_classes_of};
use crate::features4::{EncodedCandidates4, EncodedState4, StepProvenance4, TeacherTarget4};
use crate::search_labels::macro_items;

/// The head's metadata contract.
pub const HEAD_CONTRACT: &str = "search_advantage_head_v1";
/// The tail of a row past the two embeddings: the item's coarse class one
/// hot ([`COARSE_CLASSES`]), whether the payable menu added it, and whether
/// it is the seat's own answer.
pub const ROW_TAIL: usize = COARSE_CLASSES.len() + 2;
/// The most macro candidates a root compares, the policy's own included:
/// the search's `K` in the collections the head learnt from.
pub const DEFAULT_CANDIDATES: usize = 6;

/// A trained head: `advantage.onnx` (input `rows` `[N, 2d + 8]`, output
/// `score` `[N]`) beside `advantage.json`. It runs on the CPU, one session
/// per process, so a game's answers do not depend on which worker asked.
#[derive(Debug)]
pub struct AdvantageHead4 {
    session: Mutex<Session>,
    d_model: usize,
    margin: f32,
    source_checkpoint: String,
    meta: Value,
    sha256: String,
}

impl AdvantageHead4 {
    /// Loads `dir/advantage.onnx` with its `advantage.json`.
    ///
    /// # Errors
    /// Missing files, another contract or row width, a graph whose digest
    /// is not the one its metadata names, or one that does not load.
    pub fn load(dir: &Path) -> anyhow::Result<Self> {
        let meta_path = dir.join("advantage.json");
        let meta: Value = serde_json::from_slice(
            &std::fs::read(&meta_path)
                .with_context(|| format!("reading {}", meta_path.display()))?,
        )
        .with_context(|| format!("parsing {}", meta_path.display()))?;
        anyhow::ensure!(
            meta["contract"] == HEAD_CONTRACT,
            "advantage.json is not a {HEAD_CONTRACT} head"
        );
        let d_model = usize::try_from(meta["d_model"].as_u64().context("missing d_model")?)?;
        anyhow::ensure!(
            meta["row_tail"].as_u64() == Some(ROW_TAIL as u64)
                && meta["width"].as_u64() == Some((2 * d_model + ROW_TAIL) as u64),
            "the head reads rows of another width"
        );
        let margin = meta["margin"].as_f64().context("missing margin")?;
        anyhow::ensure!(
            margin.is_finite() && margin >= 0.0,
            "the head's margin must be finite and not negative"
        );
        let source_checkpoint = meta["source_checkpoint_sha256"]
            .as_str()
            .context("missing source_checkpoint_sha256")?
            .to_owned();
        let path = dir.join("advantage.onnx");
        let bytes = std::fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
        let sha256 = super::tactician::look::hex(&<sha2::Sha256 as sha2::Digest>::digest(&bytes));
        anyhow::ensure!(
            meta["onnx_sha256"].as_str() == Some(sha256.as_str()),
            "advantage.onnx is not the graph advantage.json names"
        );
        let session = Session::builder(crate::onnx::environment().map_err(ort_error)?)
            .map_err(ort_error)?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(ort_error)?
            .with_intra_threads(1)
            .map_err(ort_error)?
            .commit_from_memory(&bytes)
            .map_err(ort_error)?;
        #[allow(
            clippy::cast_possible_truncation,
            reason = "A margin in score units, read from JSON"
        )]
        let margin = margin as f32;
        Ok(Self {
            session: Mutex::new(session),
            d_model,
            margin,
            source_checkpoint,
            meta,
            sha256,
        })
    }

    /// Whether this head was fit on the embeddings of the net `identity`
    /// names: its width and its source checkpoint.
    ///
    /// # Errors
    /// Another width or another checkpoint.
    pub fn check_net(&self, identity: &ModelIdentity, d_model: usize) -> anyhow::Result<()> {
        anyhow::ensure!(
            d_model == self.d_model,
            "the head reads embeddings of width {}, the net gives {d_model}",
            self.d_model
        );
        anyhow::ensure!(
            identity.source_checkpoint_sha256.as_deref() == Some(self.source_checkpoint.as_str()),
            "the head was fit on another checkpoint's embeddings ({})",
            self.source_checkpoint
        );
        Ok(())
    }

    /// The margin a score must clear over the seat's own answer's.
    #[must_use]
    pub const fn margin(&self) -> f32 {
        self.margin
    }

    /// Its metadata and digest, for a run's record.
    #[must_use]
    pub fn identity(&self) -> Value {
        serde_json::json!({"meta": self.meta, "sha256": self.sha256})
    }

    /// One score per row of `rows` (`n` rows of [`rows`]' width).
    ///
    /// # Errors
    /// Rows of another width, or the session's failure.
    pub fn score(&self, rows: Vec<f32>, n: usize) -> anyhow::Result<Vec<f32>> {
        let width = 2 * self.d_model + ROW_TAIL;
        anyhow::ensure!(rows.len() == n * width, "student rows of the wrong width");
        let input = Tensor::from_array(([n, width], rows)).map_err(ort_error)?;
        let mut session = self
            .session
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let outputs = session
            .run(ort::inputs!["rows" => input])
            .map_err(ort_error)?;
        let (_, score) = outputs["score"]
            .try_extract_tensor::<f32>()
            .map_err(ort_error)?;
        anyhow::ensure!(
            score.len() == n && score.iter().all(|s| s.is_finite()),
            "the head answered other or non-finite scores"
        );
        Ok(score.to_vec())
    }
}

/// The head's rows, one per item of `items` in order: `[state | the item's
/// candidate embedding | its coarse class one hot, payable-added, own]`.
/// `embeddings` are the payable pass's, `classes` its menu's coarse classes,
/// `first_added` its first added item and `own` the seat's answer's item.
#[must_use]
pub fn rows(
    embeddings: &StepEmbeddings4,
    classes: &[u8],
    first_added: usize,
    own: usize,
    items: &[usize],
) -> Vec<f32> {
    let d = embeddings.state.len();
    let mut rows = Vec::with_capacity(items.len() * (2 * d + ROW_TAIL));
    for &item in items {
        rows.extend_from_slice(&embeddings.state);
        rows.extend_from_slice(&embeddings.candidates[item * d..(item + 1) * d]);
        let class = classes.get(item).copied().unwrap_or(NO_CLASS);
        rows.extend(
            (0..COARSE_CLASSES.len()).map(|at| f32::from(u8::from(usize::from(class) == at))),
        );
        rows.push(f32::from(u8::from(item >= first_added)));
        rows.push(f32::from(u8::from(item == own)));
    }
    rows
}

/// The scored item to play in place of the seat's own (`scores[0]`): the
/// best of the others when it is above the seat's by more than `margin`;
/// `None` keeps the seat's answer, ties included.
#[must_use]
pub fn choice(scores: &[f32], margin: f32) -> Option<usize> {
    let own = *scores.first()?;
    let (best, score) = scores.iter().copied().enumerate().skip(1).fold(
        None::<(usize, f32)>,
        |best, (at, score)| match best {
            Some((_, top)) if top >= score => best,
            _ => Some((at, score)),
        },
    )?;
    (score - own > margin).then_some(best)
}

/// What the student did, by count.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct StudentStats4 {
    /// Macro roots the student read.
    pub roots: u64,
    /// Of those, roots whose candidates it scored.
    pub scored: u64,
    /// Roots left to the seat, by why (`unmapped`, `no_alternative`, …).
    pub left: BTreeMap<String, u64>,
    /// Answers it played in the seat's place, by `own class -> its class`.
    pub overrides: BTreeMap<String, u64>,
    /// Planned casts it began.
    pub plans: u64,
    /// The taps their plans made, the first ones included.
    pub taps: u64,
    /// The casts sent once paid.
    pub casts: u64,
    /// Plans that broke at the table, or whose cast was not offered once
    /// paid.
    pub broken: u64,
    /// Candidates it set aside because the engine took them back from the
    /// same position ([`super::search::SearchConfig4::with_stall_guard`]'s
    /// rule).
    pub taken_back: u64,
}

impl StudentStats4 {
    /// Adds `other`'s counts to these.
    pub fn add(&mut self, other: &Self) {
        self.roots += other.roots;
        self.scored += other.scored;
        for (why, n) in &other.left {
            *self.left.entry(why.clone()).or_default() += n;
        }
        for (change, n) in &other.overrides {
            *self.overrides.entry(change.clone()).or_default() += n;
        }
        self.plans += other.plans;
        self.taps += other.taps;
        self.casts += other.casts;
        self.broken += other.broken;
        self.taken_back += other.taken_back;
    }
}

/// What scores a root's [`rows`]: the trained head ([`AdvantageHead4`]), or
/// a stand-in.
pub trait StudentHead4: std::fmt::Debug + Send + Sync {
    /// One score per row of `rows` (`n` rows).
    ///
    /// # Errors
    /// Rows of another width, or the head's failure.
    fn score(&self, rows: Vec<f32>, n: usize) -> anyhow::Result<Vec<f32>>;
    /// The margin a score must clear over the seat's own answer's.
    fn margin(&self) -> f32;
    /// Its metadata and digest, for a run's record.
    fn identity(&self) -> Value;
    /// Whether it reads the embeddings of the net `identity` names.
    ///
    /// # Errors
    /// Another width or another checkpoint.
    fn check_net(&self, identity: &ModelIdentity, d_model: usize) -> anyhow::Result<()>;
}

impl StudentHead4 for AdvantageHead4 {
    fn score(&self, rows: Vec<f32>, n: usize) -> anyhow::Result<Vec<f32>> {
        Self::score(self, rows, n)
    }
    fn margin(&self) -> f32 {
        Self::margin(self)
    }
    fn identity(&self) -> Value {
        Self::identity(self)
    }
    fn check_net(&self, identity: &ModelIdentity, d_model: usize) -> anyhow::Result<()> {
        Self::check_net(self, identity, d_model)
    }
}

/// The student a seat plays with: its head, how many macro candidates a
/// root compares, the planned cast it is paying for, and its stall memory.
#[derive(Debug)]
pub struct SearchStudent4 {
    head: Arc<dyn StudentHead4>,
    candidates: usize,
    plan: Option<PayableMenu4>,
    /// The overrides it gave from one position: asked again from there by a
    /// new question, the engine took them back.
    given: StallMemory4,
    stats: StudentStats4,
}

/// A macro root the student reads: its candidates (the seat's own first,
/// those the table took back from here set aside) and its position.
pub(super) struct StudentRoot4 {
    candidates: Vec<super::search::Candidate4>,
    position: u64,
}

impl SearchStudent4 {
    /// A student with `head`, comparing up to `candidates` macro candidates.
    ///
    /// # Errors
    /// Fewer than two candidates.
    pub fn new(head: Arc<dyn StudentHead4>, candidates: usize) -> anyhow::Result<Self> {
        anyhow::ensure!(
            candidates >= 2,
            "a student compares two candidates at least"
        );
        Ok(Self {
            head,
            candidates,
            plan: None,
            given: StallMemory4::default(),
            stats: StudentStats4::default(),
        })
    }

    /// The counts since this was last asked.
    pub fn take_stats(&mut self) -> StudentStats4 {
        std::mem::take(&mut self.stats)
    }

    /// Forgets the game: a plan and a stall memory do not outlive it.
    pub(super) fn reset(&mut self) {
        self.plan = None;
        self.given = StallMemory4::default();
    }

    /// The macro root at `view`'s question, which the seat answers `own`;
    /// `None` where the question is no macro root or leaves nothing to
    /// compare.
    pub(super) fn root(
        &mut self,
        view: &PlayerView,
        pending: &Pending,
        own: &PlayerAction,
    ) -> Option<StudentRoot4> {
        macro_root_skip(view, pending, own)
            .is_none()
            .then_some(())?;
        self.stats.roots += 1;
        let decision = RootDecision4 {
            answer: Answer4 {
                action: own.clone(),
                value: 0.5,
                provenance: StepProvenance4::default(),
                target: TeacherTarget4::default(),
            },
            ranking: None,
            notes_answer: true,
        };
        let position = HeuristicAgent::position(view);
        let taken_back = self.given.taken_back(view.seat, position, view.seq);
        let offered = macro_candidates(view, pending, &decision, self.candidates);
        let offered_count = offered.len();
        let candidates: Vec<_> = offered
            .into_iter()
            .enumerate()
            .filter(|(at, candidate)| *at == 0 || !taken_back.contains(&candidate.root.action))
            .map(|(_, candidate)| candidate)
            .collect();
        self.stats.taken_back += (offered_count - candidates.len()) as u64;
        if candidates.len() < 2 {
            self.leave("no_alternative");
            return None;
        }
        Some(StudentRoot4 {
            candidates,
            position,
        })
    }

    /// The answer it plays at `root` in place of the seat's own, read off
    /// `pass`, the payable menu there; `None` keeps the seat's answer. A
    /// chosen planned cast begins its plan, whose first tap is the answer.
    pub(super) fn decide(
        &mut self,
        view: &PlayerView,
        pending: &Pending,
        root: &StudentRoot4,
        pass: PayablePass4,
    ) -> Option<PlayerAction> {
        let candidates = &root.candidates;
        let first_added = pass.menu.first_added();
        let items = match macro_items(
            &pass.cands,
            pending,
            first_added,
            candidates.iter().map(|candidate| {
                (
                    &candidate.root.action,
                    candidate.script.as_ref().map(super::Script4::then),
                )
            }),
        ) {
            Ok(items) => items,
            Err(error) => {
                self.leave(error.key());
                return None;
            }
        };
        let classes = coarse_classes_of(&pass.cands, view);
        let rows = rows(&pass.embeddings, &classes, first_added, items[0], &items);
        let Ok(scores) = self.head.score(rows, items.len()) else {
            self.leave("head_failed");
            return None;
        };
        self.stats.scored += 1;
        let at = choice(&scores, self.head.margin())?;
        let chosen = &candidates[at];
        let action = match &chosen.script {
            Some(script) => {
                let mut menu = pass.menu;
                let Some(first) = menu.begin(view, script.then()) else {
                    self.leave("plan_refused");
                    return None;
                };
                self.plan = Some(menu);
                self.stats.plans += 1;
                first
            }
            None => chosen.root.action.clone(),
        };
        self.given.give(view.seat, root.position, view.seq, &action);
        let name = |item: usize| {
            classes
                .get(item)
                .and_then(|class| COARSE_CLASSES.get(usize::from(*class)))
                .copied()
                .unwrap_or("none")
        };
        let label = if chosen.script.is_some() {
            format!("{}->planned_cast", name(items[0]))
        } else {
            format!("{}->{}", name(items[0]), name(items[at]))
        };
        *self.stats.overrides.entry(label).or_default() += 1;
        Some(action)
    }

    /// The next answer of the planned cast being paid, when this question is
    /// the plan's own; a plan that broke or is done ends.
    pub(super) fn next(&mut self, view: &PlayerView, pending: &Pending) -> Option<PlayerAction> {
        let plan = self.plan.as_mut()?;
        let answer = plan.next(view, pending);
        if !plan.paying() {
            // The plan's own counts: its taps (the first one included), the
            // cast sent, and a break or a cast not offered once paid.
            let counted = plan.take_stats();
            self.stats.taps += counted.taps;
            self.stats.casts += counted.cast;
            self.stats.broken += counted.broken + counted.not_castable;
            self.plan = None;
        }
        answer
    }

    fn leave(&mut self, why: &str) {
        *self.stats.left.entry(why.to_owned()).or_default() += 1;
    }
}

/// One memory-neutral pass of the net over a root's payable menu.
pub struct PayablePass4 {
    /// The menu: the seat's own, then what the payable menu added.
    pub cands: EncodedCandidates4,
    /// The payable menu, with the plan of each cast it added.
    pub menu: PayableMenu4,
    /// The graph's embeddings over the menu.
    pub embeddings: StepEmbeddings4,
}

impl NetPlayer4 {
    /// Appends the payable menu's casts to `cands`, encoded as this net
    /// encodes its menu: the one place the student's root menu is built.
    pub(super) fn extend_payable(
        &self,
        menu: &mut PayableMenu4,
        state: &EncodedState4,
        cands: &mut EncodedCandidates4,
        view: &PlayerView,
        pending: &Pending,
    ) {
        let encoder = self.encoder_version;
        menu.extend(cands, view, pending, |announced| {
            if encoder == 10 {
                crate::features10::encode_candidates10(state, view, announced)
            } else {
                crate::features7::encode_candidates7_grouped(state, view, announced)
            }
        });
    }

    /// The seat's payable menu at `view`'s question and the graph's
    /// embeddings over it, from the seat's memory, which a decision step
    /// never changes; nothing of this player changes.
    ///
    /// # Errors
    /// A graph exported without embeddings, or the step's failure.
    pub fn payable_pass(
        &mut self,
        view: &PlayerView,
        pending: &Pending,
        teams: &[Option<u8>],
    ) -> anyhow::Result<PayablePass4> {
        let h_in = self
            .h_states
            .get(&view.seat.get())
            .cloned()
            .unwrap_or_else(|| vec![0.0; RECURRENT_SLOTS * self.d_model]);
        let (state, mut cands) = self.policy_encoding(view, pending, teams);
        let mut menu = PayableMenu4::new(0.0);
        self.extend_payable(&mut menu, &state, &mut cands, view, pending);
        let outputs = self.run_onnx_step(&state, &cands, &h_in, self.thinking_steps)?;
        let embeddings = outputs
            .embeddings
            .context("the net's graph exports no embeddings (export it with --embeddings)")?;
        Ok(PayablePass4 {
            cands,
            menu,
            embeddings,
        })
    }

    /// Sets the search student: from the next decision on, it reads every
    /// macro root of this player's seats.
    ///
    /// # Errors
    /// A head fit on another net's embeddings.
    pub fn set_search_student(&mut self, student: SearchStudent4) -> anyhow::Result<()> {
        student.head.check_net(&self.model_identity, self.d_model)?;
        self.search_student = Some(student);
        Ok(())
    }

    /// The student's head and how many candidates it compares, for a
    /// record's header; `None` without one.
    #[must_use]
    pub fn search_student_identity(&self) -> Option<Value> {
        self.search_student.as_ref().map(|student| {
            serde_json::json!({
                "contract": HEAD_CONTRACT,
                "head": student.head.identity(),
                "margin": student.head.margin(),
                "candidates": student.candidates,
            })
        })
    }

    /// What the student did since this was last asked.
    pub fn take_student_stats(&mut self) -> Option<StudentStats4> {
        self.search_student.as_mut().map(SearchStudent4::take_stats)
    }

    /// The student's answer in place of `own`, the seat's answer at
    /// `view`'s question, where it plays another; `None` keeps `own`.
    pub(super) fn student_answer(
        &mut self,
        view: &PlayerView,
        pending: &Pending,
        teams: &[Option<u8>],
        own: &PlayerAction,
    ) -> Option<PlayerAction> {
        let mut student = self.search_student.take()?;
        let answer = student.root(view, pending, own).and_then(|root| {
            if let Ok(pass) = self.payable_pass(view, pending, teams) {
                student.decide(view, pending, &root, pass)
            } else {
                student.leave("pass_failed");
                None
            }
        });
        self.search_student = Some(student);
        answer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_row_is_the_state_the_item_and_its_tail() {
        let embeddings = StepEmbeddings4 {
            state: vec![1.0, 2.0],
            candidates: vec![10.0, 11.0, 20.0, 21.0, 30.0, 31.0],
            pairs: Vec::new(),
        };
        // Items: a pass (0), a tap (1), an added cast (2), the added one first.
        let rows = rows(&embeddings, &[0, 1, 2], 2, 1, &[1, 2, 0]);
        let width = 2 * 2 + ROW_TAIL;
        assert_eq!(rows.len(), 3 * width);
        let tap = &rows[..width];
        assert_eq!(&tap[..4], &[1.0, 2.0, 20.0, 21.0]);
        assert_eq!(
            &tap[4..],
            &[0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0],
            "a tap, own"
        );
        let cast = &rows[width..2 * width];
        assert_eq!(&cast[2..4], &[30.0, 31.0]);
        assert_eq!(
            &cast[4..],
            &[0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0],
            "added cast"
        );
        let pass = &rows[2 * width..];
        assert_eq!(&pass[4..], &[1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
        let unknown = rows_for_class(NO_CLASS);
        assert!(
            unknown[4..4 + COARSE_CLASSES.len()]
                .iter()
                .all(|x| *x == 0.0)
        );
    }

    fn rows_for_class(class: u8) -> Vec<f32> {
        let embeddings = StepEmbeddings4 {
            state: vec![0.0, 0.0],
            candidates: vec![0.0, 0.0],
            pairs: Vec::new(),
        };
        rows(&embeddings, &[class], 1, 0, &[0])
    }

    #[test]
    fn the_student_plays_another_answer_only_past_its_margin() {
        assert_eq!(choice(&[0.10, 0.12, 0.30], 0.05), Some(2));
        assert_eq!(choice(&[0.10, 0.14, 0.12], 0.05), None, "inside the margin");
        assert_eq!(
            choice(&[0.25, 0.5], 0.25),
            None,
            "a gain equal to the margin keeps"
        );
        assert_eq!(
            choice(&[0.10, 0.30, 0.30], 0.05),
            Some(1),
            "ties to the earlier"
        );
        assert_eq!(choice(&[0.10], 0.0), None);
        assert_eq!(choice(&[], 0.0), None);
    }

    /// A stand-in head: an item the payable menu added scores 1, any other
    /// 0.
    #[derive(Debug)]
    struct PrefersAdded {
        margin: f32,
    }

    impl StudentHead4 for PrefersAdded {
        fn score(&self, rows: Vec<f32>, n: usize) -> anyhow::Result<Vec<f32>> {
            let width = rows.len() / n;
            Ok(rows.chunks(width).map(|row| row[width - 2]).collect())
        }
        fn margin(&self) -> f32 {
            self.margin
        }
        fn identity(&self) -> Value {
            Value::Null
        }
        fn check_net(&self, _: &ModelIdentity, _: usize) -> anyhow::Result<()> {
            Ok(())
        }
    }

    fn identity(name: &str) -> baylee_view::CardIdentity {
        baylee_view::CardIdentity {
            index: baylee_cards::decks::by_name(name).expect(name),
            print: baylee_core::ids::PrintRef::new(0),
            face: 0,
        }
    }

    /// An untapped Island and Opt in hand at the start of a chain, the seat
    /// about to tap the Island: the engine offers Opt as payable once the
    /// Island is tapped.
    fn island_and_opt() -> (PlayerView, Pending, PlayerAction) {
        use baylee_core::ids::{ObjectId, PlayerId};
        let card = identity("Island");
        let face = &baylee_cards::by_index(card.index).expect("Island").faces[0];
        let mut island = baylee_client_core::test_support::token(70, 0, "Island", 0, 0);
        island.card = Some(card);
        island.rules = Some(card.into());
        island.types = face.types;
        island.supertypes = face.supertypes;
        island.subtypes = baylee_core::types::SubtypeSet::from_slice(face.subtypes);
        island.power = None;
        island.toughness = None;
        let mut view = baylee_client_core::test_support::ViewBuilder::new(2)
            .with_battlefield(0, vec![island])
            .build();
        let opt = ObjectId::new(61, 0);
        view.hand = vec![baylee_view::HandObject {
            id: opt,
            card: identity("Opt"),
            name: "Opt".into(),
            mana_value: 1,
            colors: baylee_core::color::ColorSet::default(),
            types: baylee_core::types::TypeSet::INSTANT,
            commander: false,
        }];
        let island = ObjectId::new(70, 0);
        let pending = Pending::Priority {
            player: PlayerId::new(0),
            legal: Box::new(baylee_engine::choice::LegalActions {
                can_pass: true,
                mana_abilities: vec![island],
                payable: vec![opt],
                ..baylee_engine::choice::LegalActions::default()
            }),
        };
        let tap = PlayerAction::ActivateManaAbility { source: island };
        (view, pending, tap)
    }

    /// The payable menu at a root, as the net's pass would read it, with
    /// embeddings of nothing.
    fn payable_pass_at(view: &PlayerView, pending: &Pending) -> PayablePass4 {
        let state = crate::features7::encode_state7(view, Some(pending), None, &[]);
        let mut cands = crate::features7::encode_candidates7_grouped(&state, view, pending);
        let mut menu = PayableMenu4::new(0.0);
        menu.extend(&mut cands, view, pending, |announced| {
            crate::features7::encode_candidates7_grouped(&state, view, announced)
        });
        let embeddings = StepEmbeddings4 {
            state: vec![0.0; 2],
            candidates: vec![0.0; 2 * cands.candidates.len()],
            pairs: Vec::new(),
        };
        PayablePass4 {
            cands,
            menu,
            embeddings,
        }
    }

    /// Past its margin the student plays the planned cast in the seat's
    /// place: its first tap, the rest of the plan after it; inside its
    /// margin the seat's own answer stands.
    #[test]
    fn a_planned_cast_past_the_margin_begins_its_plan() {
        let (view, pending, tap) = island_and_opt();
        let mut student =
            SearchStudent4::new(Arc::new(PrefersAdded { margin: 0.5 }), DEFAULT_CANDIDATES)
                .unwrap();
        let root = student.root(&view, &pending, &tap).expect("a macro root");
        assert_eq!(root.candidates.len(), 3, "the tap, a pass and Opt planned");
        assert!(root.candidates[2].script.is_some());
        let pass = payable_pass_at(&view, &pending);
        assert!(
            pass.menu.first_added() < pass.cands.candidates.len(),
            "the menu added Opt"
        );
        let answer = student.decide(&view, &pending, &root, pass);
        assert_eq!(answer, Some(tap.clone()), "the plan's first tap");
        assert!(student.plan.as_ref().is_some_and(PayableMenu4::paying));
        let stats = student.take_stats();
        assert_eq!((stats.roots, stats.scored, stats.plans), (1, 1, 1));
        assert_eq!(stats.overrides["tap->planned_cast"], 1);

        let mut sure = SearchStudent4::new(Arc::new(PrefersAdded { margin: 1.0 }), 6).unwrap();
        let root = sure.root(&view, &pending, &tap).expect("a macro root");
        assert_eq!(
            sure.decide(&view, &pending, &root, payable_pass_at(&view, &pending)),
            None
        );
        assert!(sure.plan.is_none());
        assert!(SearchStudent4::new(Arc::new(PrefersAdded { margin: 0.0 }), 1).is_err());
    }

    /// Mid-chain, or a pass outside the seat's main phase with nothing to
    /// answer, is no macro root: the student does not read it.
    #[test]
    fn the_student_reads_macro_roots_only() {
        let (mut view, pending, _) = island_and_opt();
        let mut student = SearchStudent4::new(Arc::new(PrefersAdded { margin: 0.0 }), 6).unwrap();
        view.active = baylee_core::ids::PlayerId::new(1);
        assert!(
            student
                .root(&view, &pending, &PlayerAction::PassPriority)
                .is_none(),
            "a pass on the other seat's turn with an empty stack"
        );
        assert_eq!(student.take_stats().roots, 0);
    }
}
