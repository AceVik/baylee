//! The search on real tables with a stand-in for the net: the plumbing,
//! not playing strength. No model is loaded.
use std::collections::BTreeMap;

use super::*;
use crate::features4::{CoverageMatrix4, HeadKind4, PendingKind4};
use crate::netplay4::rank_single_choice;
use baylee_ai::HeuristicAgent;
use baylee_core::preset::AIProfile;
use baylee_core::types::TypeSet;

pub(super) const ME: PlayerId = PlayerId::new(0);
pub(super) const SEED: u64 = 0x0b5e_55ed_5eed_0001;

/// What the stand-in changes while it plays, all of it.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct MockState {
    observed: BTreeMap<u8, f32>,
    observations: u64,
    answers: u64,
    concluded: Vec<PlayerAction>,
    sampling: u64,
    coverage: Vec<[u64; 5]>,
}

/// A stand-in for the net: its own answer at a root is `own`, its critic is
/// `value`, its rollout answers are the steady house's, and its ranking is
/// the real encoder's menu, in menu order, its own answer's logit raised by
/// `confidence`. When it `lends`, its rollout players are copies of it.
pub(super) struct Mock {
    pub(super) state: MockState,
    own: fn(&PlayerView, &Pending) -> Option<PlayerAction>,
    value: fn(&PlayerView) -> f32,
    confidence: f32,
    lends: bool,
    /// Rollout answers past this many are given as the house's, as a net
    /// gives its fallbacks.
    pub(super) fallback_after: Option<u64>,
}

impl Mock {
    pub(super) fn new(
        own: fn(&PlayerView, &Pending) -> Option<PlayerAction>,
        value: fn(&PlayerView) -> f32,
    ) -> Self {
        Self {
            state: MockState {
                coverage: CoverageMatrix4::default().counts.to_vec(),
                ..MockState::default()
            },
            own,
            value,
            confidence: 0.0,
            lends: false,
            fallback_after: None,
        }
    }

    /// The same stand-in, lending rollout players.
    fn lending(self) -> Self {
        Self {
            lends: true,
            ..self
        }
    }

    /// The same stand-in, `logit` surer of its own answer than of the rest.
    fn confident(self, logit: f32) -> Self {
        Self {
            confidence: logit,
            ..self
        }
    }

    fn record(&mut self, pending: &Pending, source: SourceKind4) {
        self.state.coverage[PendingKind4::of(pending) as usize][source as usize] += 1;
    }
}

impl SearchPlayer4 for Mock {
    type Snapshot = MockState;

    fn snapshot(&self) -> MockState {
        self.state.clone()
    }
    fn restore(&mut self, snapshot: &MockState) {
        self.state.clone_from(snapshot);
    }
    fn view_context(&self) -> ViewContext4 {
        ViewContext4::Choice
    }
    fn observe(&mut self, view: &PlayerView) -> anyhow::Result<()> {
        self.state.observations += 1;
        self.state
            .observed
            .insert(view.seat.get(), (self.value)(view));
        Ok(())
    }
    fn observed_value(&self, seat: PlayerId) -> Option<f32> {
        self.state.observed.get(&seat.get()).copied()
    }
    fn decide(
        &mut self,
        view: &PlayerView,
        pending: &Pending,
        teams: &[Option<u8>],
    ) -> RootDecision4 {
        let state = crate::features7::encode_state7(view, Some(pending), None, teams);
        let cands = crate::features7::encode_candidates7_grouped(&state, view, pending);
        let policy = cands.head == HeadKind4::SingleChoice && cands.candidates.len() > 1;
        // A declaration with a creature to declare is the policy's own too.
        let declares = combat::is_declaration(pending) && cands.candidates.len() > 1;
        let action = (self.own)(view, pending)
            .filter(|a| pending.answer_fault(a).is_none())
            .unwrap_or_else(|| HeuristicAgent::new(AIProfile::STEADY).act(view, pending));
        let source = if policy || declares {
            SourceKind4::Net
        } else {
            SourceKind4::Forced
        };
        self.record(pending, source);
        let ranking = policy.then(|| {
            let flat = vec![0.0; cands.candidates.len()];
            let mut logits = flat.clone();
            for ranked in rank_single_choice(&cands, &flat, view, pending) {
                if ranked.action == action {
                    logits[ranked.index] = self.confidence;
                }
            }
            rank_single_choice(&cands, &logits, view, pending)
        });
        RootDecision4 {
            answer: Answer4 {
                action,
                value: 0.5,
                provenance: StepProvenance4 {
                    source,
                    temperature: 0.0,
                    thinking_steps: 2,
                    behavior_logprob: 0.0,
                    has_logprob: false,
                },
                target: TeacherTarget4::default(),
            },
            ranking,
            notes_answer: policy,
        }
    }
    fn conclude(&mut self, _view: &PlayerView, decision: &RootDecision4, action: &PlayerAction) {
        if decision.notes_answer {
            self.state.concluded.push(action.clone());
        }
    }
    fn answer(&mut self, view: &PlayerView, pending: &Pending, _teams: &[Option<u8>]) -> Answer4 {
        self.state.answers += 1;
        let action = HeuristicAgent::new(AIProfile::STEADY)
            .with_seed(self.state.sampling)
            .act(view, pending);
        self.record(pending, SourceKind4::Net);
        let source = if self
            .fallback_after
            .is_some_and(|after| self.state.answers > after)
        {
            SourceKind4::House
        } else {
            SourceKind4::Net
        };
        Answer4 {
            action,
            value: 0.5,
            provenance: StepProvenance4 {
                source,
                ..StepProvenance4::default()
            },
            target: TeacherTarget4::default(),
        }
    }
    fn reseed_sampling(&mut self, seed: u64) {
        self.state.sampling = seed;
    }
    fn attribute_to_search(&mut self, pending: &Pending, from: SourceKind4) {
        let row = &mut self.state.coverage[PendingKind4::of(pending) as usize];
        row[from as usize] -= 1;
        row[SourceKind4::Search as usize] += 1;
    }
    fn lends_rollout_players(&self) -> bool {
        self.lends
    }
    fn rollout_player(&self, snapshot: &MockState) -> Option<Self> {
        self.lends.then(|| Self {
            state: snapshot.clone(),
            ..*self
        })
    }
}

/// Passes whenever it may.
pub(super) fn passes(_: &PlayerView, pending: &Pending) -> Option<PlayerAction> {
    matches!(pending, Pending::Priority { .. }).then_some(PlayerAction::PassPriority)
}

/// Plays as the steady house.
#[allow(
    clippy::unnecessary_wraps,
    reason = "One signature for every stand-in policy"
)]
pub(super) fn steady(view: &PlayerView, pending: &Pending) -> Option<PlayerAction> {
    Some(HeuristicAgent::new(AIProfile::STEADY).act(view, pending))
}

/// A flat critic.
fn flat(_: &PlayerView) -> f32 {
    0.5
}

/// A critic that counts the seat's permanents of `types`, from 0.2 up.
fn counting(view: &PlayerView, types: TypeSet) -> f32 {
    let mine = view
        .battlefield
        .iter()
        .filter(|o| o.controller == view.seat && o.types.contains(types))
        .count();
    (0.2 + 0.1 * f32::from(u16::try_from(mine).unwrap_or(u16::MAX))).min(1.0)
}

/// A critic that likes the seat's creatures.
pub(super) fn creatures(view: &PlayerView) -> f32 {
    counting(view, TypeSet::CREATURE)
}

/// A critic that likes the seat's lands: the obvious answer to a land in
/// hand is to play it.
fn lands(view: &PlayerView) -> f32 {
    counting(view, TypeSet::LAND)
}

/// Victory against Victory.
fn preset(seed: u64) -> baylee_core::preset::GamePreset {
    let deck = crate::housedeck::HouseDeck::named("victory").unwrap();
    crate::selfplay::table_for(seed, &[&deck, &deck], &[AIProfile::STEADY; 2], &[])
}

/// [`preset`], seat 0 taken over, recorded.
fn table(seed: u64) -> Session {
    seated(&preset(seed))
}

/// `preset`'s table, seat 0 taken over, recorded.
fn seated(preset: &baylee_core::preset::GamePreset) -> Session {
    let mut session = Session::new_recorded(preset, "test").unwrap();
    session.describe("search".into(), vec!["0".into(), "1".into()]);
    assert!(session.take_over(ME));
    session
}

/// Drives the table, seat 0 by the steady house, until seat 0 holds
/// priority in its own main phase with `want` true of the root and the
/// table forkable. Returns the root's question and view.
fn root(session: &mut Session, want: fn(&PlayerView, &Pending) -> bool) -> (Pending, PlayerView) {
    for _ in 0..5000 {
        assert!(!matches!(session.pending(), Pending::GameOver(_)));
        if session.needs_pump() {
            session.pump_at_most(1);
            continue;
        }
        let (pending, view) = session.view_for_with_context(ME).unwrap();
        if matches!(pending, Pending::Priority { .. })
            && view.active == ME
            && matches!(view.phase, Phase::FirstMain)
            && want(&view, &pending)
            && KnownDecks(session).fork(ME, 1).is_ok()
        {
            return (pending, view);
        }
        let action = session.house_action(ME).unwrap();
        session.act_slice(ME, action, 0).unwrap();
    }
    panic!("no such root");
}

/// A land drop is among the legal answers, past the opening turns.
fn land_playable(view: &PlayerView, pending: &Pending) -> bool {
    view.turn >= 3 && matches!(pending, Pending::Priority { legal, .. } if !legal.lands.is_empty())
}

fn any_root(view: &PlayerView, _: &Pending) -> bool {
    view.turn >= 3
}

pub(super) fn config(k: usize, p: usize, h: usize) -> SearchConfig4 {
    SearchConfig4::new(k, p, h).unwrap()
}

/// A table that refuses every fork, as a position the engine cannot fork.
struct Refusing<'a>(&'a Session);
impl ForkSource4 for Refusing<'_> {
    fn fork(&self, _: PlayerId, _: u64) -> Result<SimulationSession, ForkRefusal> {
        Err(ForkRefusal::FaceDown)
    }
    fn decision_seq(&self) -> u64 {
        self.0.decision_seq()
    }
}

#[test]
fn one_candidate_plays_the_policys_answer_and_every_world_is_undone() {
    let mut searched_table = table(21);
    let mut plain_table = table(21);
    let (pending, view) = root(&mut searched_table, any_root);
    let _ = root(&mut plain_table, any_root);
    let hash = searched_table.snapshot_hash();
    let recorded = searched_table.record_pending();

    let mut searched = Mock::new(steady, creatures);
    let mut plain = Mock::new(steady, creatures);
    searched.observe(&view).unwrap();
    plain.observe(&view).unwrap();
    let result = search_decision(
        &mut searched,
        &KnownDecks(&searched_table),
        &view,
        &pending,
        &[],
        config(1, 3, 12),
        SEED,
    )
    .unwrap();
    let decision = plain.decide(&view, &pending, &[]);
    plain.conclude(&view, &decision, &decision.answer.action);

    let SearchOutcome4::Searched(report) = &result.outcome else {
        panic!("the root is searched: {:?}", result.outcome);
    };
    assert_eq!((report.candidates, report.rounds), (1, 3));
    assert_eq!(
        report.actions,
        std::slice::from_ref(&decision.answer.action)
    );
    assert!(!report.changed);
    assert_eq!(report.rollouts, 3);
    assert_eq!(report.leaves.terminal + report.leaves.value(), 3);
    assert_eq!(report.leaves.immediate, 0, "H > 0 plays the worlds on");
    assert_eq!(result.answer.action, decision.answer.action);
    assert_eq!(result.answer.provenance.source, SourceKind4::Search);
    assert!(!result.answer.provenance.has_logprob);
    assert_eq!(result.answer.provenance.behavior_logprob, 0.0);

    // Every world undone: the stand-in is where answering would have left
    // it, except that its coverage counts the decision as searched.
    let kind = PendingKind4::of(&pending) as usize;
    let mut expected = plain.state.clone();
    expected.coverage[kind][SourceKind4::Net as usize] -= 1;
    expected.coverage[kind][SourceKind4::Search as usize] += 1;
    assert_eq!(searched.state, expected);
    assert_eq!(searched_table.snapshot_hash(), hash);
    assert_eq!(searched_table.record_pending(), recorded);

    searched_table
        .act_slice(ME, result.answer.action, 0)
        .unwrap();
    plain_table
        .act_slice(ME, decision.answer.action, 0)
        .unwrap();
    assert_eq!(searched_table.snapshot_hash(), plain_table.snapshot_hash());
    assert_eq!(searched_table.take_record(), plain_table.take_record());
}

/// A critic that likes the seat's lands only a little.
fn lands_a_little(view: &PlayerView) -> f32 {
    0.2 + (lands(view) - 0.2) / 10.0
}

/// The land-drop root of table 22, searched by a stand-in that passes
/// there, under `selection`.
fn land_root_search(
    critic: fn(&PlayerView) -> f32,
    confidence: f32,
    k: usize,
    selection: Selection4,
) -> (Mock, Pending, SearchedAnswer4) {
    let mut session = table(22);
    let (pending, view) = root(&mut session, land_playable);
    let mut player = Mock::new(passes, critic).confident(confidence);
    player.observe(&view).unwrap();
    let result = search_decision(
        &mut player,
        &KnownDecks(&session),
        &view,
        &pending,
        &[],
        config(k, 1, 0).with_selection(selection).unwrap(),
        SEED,
    )
    .unwrap();
    session
        .act_slice(ME, result.answer.action.clone(), 0)
        .expect("the searched answer is legal at the table");
    (player, pending, result)
}

#[test]
fn a_value_lookahead_finds_the_land_drop_the_policy_passed_on() {
    let paired = Selection4::Paired {
        margin: 0.05,
        z: 0.0,
    };
    let (player, pending, result) =
        land_root_search(lands, 0.0, SearchConfig4::MAX_CANDIDATES, paired);
    let SearchOutcome4::Searched(report) = &result.outcome else {
        panic!("the root is searched: {:?}", result.outcome);
    };
    assert!(report.changed, "{report:?}");
    assert_eq!(report.leaves.immediate, report.rollouts);
    assert_eq!(
        report.rollout_steps, 0,
        "H = 0 scores the answer's own position"
    );
    assert!(report.means[report.chosen] > report.means[0]);
    assert!((report.diffs[report.chosen] - 0.1).abs() < 1e-6);
    assert_eq!(
        report.ses[report.chosen], None,
        "one world, no standard error"
    );
    assert_eq!(report.actions[0], PlayerAction::PassPriority);
    assert_eq!(report.actions[report.chosen], result.answer.action);
    assert_eq!(report.actions.len(), report.candidates);
    let PlayerAction::PlayLand { card } = result.answer.action else {
        panic!("a land drop: {:?}", result.answer.action);
    };
    let Pending::Priority { legal, .. } = &pending else {
        unreachable!("the root is priority");
    };
    assert!(legal.lands.contains(&card));
    assert_eq!(
        player.state.concluded.last(),
        Some(&result.answer.action),
        "the player notes the searched answer as its own"
    );
}

/// The land-drop root of table 22 searched as [`land_root_search`] does
/// with `lands` and a paired margin, never choosing an answer of
/// `taken_back`.
fn land_root_avoiding(taken_back: &[PlayerAction]) -> SearchedAnswer4 {
    let mut session = table(22);
    let (pending, view) = root(&mut session, land_playable);
    let mut player = Mock::new(passes, lands);
    player.observe(&view).unwrap();
    let paired = Selection4::Paired {
        margin: 0.05,
        z: 0.0,
    };
    let config = config(SearchConfig4::MAX_CANDIDATES, 1, 0)
        .with_selection(paired)
        .unwrap()
        .with_stall_guard(true);
    let result = search_decision_avoiding(
        &mut player,
        &KnownDecks(&session),
        &view,
        &pending,
        &[],
        config,
        SEED,
        taken_back,
    )
    .unwrap();
    session
        .act_slice(ME, result.answer.action.clone(), 0)
        .expect("the searched answer is legal at the table");
    result
}

/// An answer the table took back from this position is not compared
/// again; when it is the policy's own, the best of the others is played
/// even inside the margin.
#[test]
fn the_stall_guard_never_chooses_an_answer_the_table_took_back() {
    let free = land_root_avoiding(&[]);
    let SearchOutcome4::Searched(report) = &free.outcome else {
        panic!("searched: {:?}", free.outcome);
    };
    assert!(report.changed && report.taken_back.is_empty());
    let land = free.answer.action.clone();
    assert!(matches!(land, PlayerAction::PlayLand { .. }));

    assert_eq!(report.actions.len(), 2, "a pass and one land: {report:?}");

    // The land taken back: nothing is left to compare with the pass, which
    // the policy plays unsearched.
    let guarded = land_root_avoiding(std::slice::from_ref(&land));
    assert_eq!(
        guarded.outcome,
        SearchOutcome4::Skipped(SearchSkip4::TakenBack)
    );
    assert_eq!(guarded.answer.action, PlayerAction::PassPriority);

    // The policy's pass taken back: the land is played though the pass is
    // still rolled out beside it.
    let pass = land_root_avoiding(&[PlayerAction::PassPriority]);
    let SearchOutcome4::Searched(report) = &pass.outcome else {
        panic!("searched: {:?}", pass.outcome);
    };
    assert_eq!(report.taken_back, [PlayerAction::PassPriority]);
    assert_eq!(report.actions, [PlayerAction::PassPriority, land.clone()]);
    assert!(report.changed);
    assert_eq!(pass.answer.action, land);
    assert_eq!(report.ends.len(), report.rounds);
    assert!(
        report
            .ends
            .iter()
            .all(|round| round.len() == report.candidates
                && round.iter().all(|end| *end == LeafEnd4::Immediate)),
        "H = 0 ends every rollout at once: {:?}",
        report.ends
    );
}

/// The candidates left once the answers taken back are set aside: every
/// other one with such an answer goes (a cast tried at several targets
/// with it), the policy's own stays first and is named.
#[test]
fn the_answers_taken_back_are_set_aside_but_the_policys_own() {
    let card = |n| baylee_core::ids::ObjectId::new(n, 0);
    let plain = |index, action| {
        Candidate4::plain(Ranked4 {
            index,
            action,
            logit: 0.0,
        })
    };
    let cast = PlayerAction::CastSpell { card: card(3) };
    let land = PlayerAction::PlayLand { card: card(4) };
    let at = |target: u32| Candidate4 {
        target: Some(TargetFollow4 {
            action: PlayerAction::PassPriority,
            prior: f64::from(target),
        }),
        ..plain(2, cast.clone())
    };
    let candidates = || {
        vec![
            plain(0, PlayerAction::PassPriority),
            at(1),
            at(2),
            plain(3, land.clone()),
        ]
    };
    let none = Avoided::drop(candidates(), &[]);
    assert_eq!(none.candidates, candidates());
    assert!(none.taken_back.is_empty() && !none.own);

    let some = Avoided::drop(candidates(), &[cast.clone(), land.clone()]);
    assert_eq!(some.candidates, [plain(0, PlayerAction::PassPriority)]);
    assert_eq!(some.taken_back, [cast.clone(), land.clone()]);
    assert!(!some.own);

    let own = Avoided::drop(candidates(), &[PlayerAction::PassPriority]);
    assert_eq!(own.candidates, candidates());
    assert_eq!(own.taken_back, [PlayerAction::PassPriority]);
    assert!(own.own);
}

#[test]
fn a_gain_inside_the_margin_leaves_the_policys_answer() {
    let k = SearchConfig4::MAX_CANDIDATES;
    let paired = Selection4::Paired {
        margin: 0.05,
        z: 0.0,
    };
    let changed = |selection| {
        let (_, _, result) = land_root_search(lands_a_little, 0.0, k, selection);
        let SearchOutcome4::Searched(report) = result.outcome else {
            panic!("searched");
        };
        report.changed
    };
    assert!(changed(Selection4::Mean), "v1 takes a gain of 0.01");
    assert!(!changed(paired), "0.01 is inside a margin of 0.05");
    assert!(
        !changed(Selection4::Paired {
            margin: 0.05,
            z: 1.0
        }),
        "one world cannot clear a standard error"
    );
}

#[test]
fn the_prior_keeps_a_confident_policys_answer() {
    let k = SearchConfig4::MAX_CANDIDATES;
    let (_, _, sure) = land_root_search(lands, 6.0, k, Selection4::Prior { c: 1.0 });
    let SearchOutcome4::Searched(report) = &sure.outcome else {
        panic!("searched");
    };
    assert!(report.priors[0] > 0.9, "{:?}", report.priors);
    assert!(
        !report.changed,
        "a gain of 0.1 against a prior bonus near 0.45"
    );
    assert!((report.scores[0] - report.means[0] - report.priors[0] / 2.0).abs() < 1e-9);
    let (_, _, unsure) = land_root_search(lands, 0.0, k, Selection4::Prior { c: 1.0 });
    let SearchOutcome4::Searched(report) = &unsure.outcome else {
        panic!("searched");
    };
    assert!(report.changed, "uniform priors leave the mean to decide");
}

#[test]
fn a_rollout_scores_a_settled_position_and_counts_only_the_seats_choices() {
    let mut session = table(24);
    let (pending, view) = root(&mut session, land_playable);
    assert!(
        rollout::settled(&view, &pending),
        "the root itself is settled"
    );
    let Pending::Priority { legal, .. } = &pending else {
        unreachable!("the root is priority");
    };
    let mut fork = KnownDecks(&session).fork(ME, 5).unwrap();
    // A basic land type's mana ability, or one a card prints.
    let tap = legal.mana_abilities.first().map_or_else(
        || {
            let &(source, ability_index) = legal
                .abilities
                .iter()
                .find(|&&(source, index)| {
                    crate::netplay4::tactics::is_mana_ability_entry(&view, source, index)
                })
                .expect("a mana source to tap");
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            }
        },
        |&source| PlayerAction::ActivateManaAbility { source },
    );
    fork.act_slice(ME, tap.clone(), 0).unwrap();
    let (after, tapped) = fork.view_for_with_context(ME).unwrap();
    assert!(!rollout::settled(&tapped, &after), "mana is floating");
    let answer = |action, source| Answer4 {
        action,
        value: 0.5,
        provenance: StepProvenance4 {
            source,
            ..StepProvenance4::default()
        },
        target: TeacherTarget4::default(),
    };
    assert!(!rollout::counts_toward_horizon(
        &answer(tap, SourceKind4::Net),
        &view
    ));
    assert!(!rollout::counts_toward_horizon(
        &answer(PlayerAction::PassPriority, SourceKind4::Forced),
        &view
    ));
    assert!(rollout::counts_toward_horizon(
        &answer(PlayerAction::PassPriority, SourceKind4::Net),
        &view
    ));

    let mut player = Mock::new(steady, lands);
    player.observe(&view).unwrap();
    let horizon = 8;
    let result = search_decision(
        &mut player,
        &KnownDecks(&session),
        &view,
        &pending,
        &[],
        config(4, 2, horizon),
        SEED,
    )
    .unwrap();
    let SearchOutcome4::Searched(report) = &result.outcome else {
        panic!("searched: {:?}", result.outcome);
    };
    let leaves = report.leaves;
    assert_eq!(leaves.terminal + leaves.value(), report.rollouts);
    assert!(leaves.settled > 0, "{leaves:?}");
    assert_eq!(leaves.immediate + leaves.stopped, 0, "{leaves:?}");
    assert!(report.seat_decisions <= report.rollouts * horizon as u64);
}

#[test]
fn a_search_is_a_function_of_its_inputs() {
    let run = |seed| {
        let mut session = table(23);
        let (pending, view) = root(&mut session, land_playable);
        let mut player = Mock::new(passes, creatures);
        player.observe(&view).unwrap();
        let result = search_decision(
            &mut player,
            &KnownDecks(&session),
            &view,
            &pending,
            &[],
            config(4, 2, 16),
            seed,
        )
        .unwrap();
        (result.answer.action, result.outcome, player.state)
    };
    let first = run(SEED);
    assert_eq!(first, run(SEED));
    let SearchOutcome4::Searched(report) = &first.1 else {
        panic!("searched");
    };
    assert_eq!(report.rounds, 2);
    assert_eq!(report.rollouts, 2 * report.candidates as u64);
    assert!(report.rollout_steps > 0);
}

/// [`preset`] with my opening hand fixed, three lands and the first four
/// other cards of my list, and everything else dealt from `seed`: their
/// hand and both libraries.
fn fixed_hand_preset(seed: u64) -> baylee_core::preset::GamePreset {
    let mut preset = preset(seed);
    let mine = &mut preset.seats[usize::from(ME.get())];
    let is_land = |entry: &&baylee_core::preset::DeckEntry| {
        baylee_cards::by_index(entry.card)
            .is_some_and(|def| def.faces[0].types.contains(TypeSet::LAND))
    };
    let hand: Vec<_> = (mine.deck.iter().filter(is_land).take(3))
        .chain(mine.deck.iter().filter(|e| !is_land(e)).take(4))
        .copied()
        .collect();
    assert_eq!(hand.len(), 7);
    mine.starting_hand = Some(hand);
    preset
}

/// My first priority in my first main phase of the game, the table driven
/// there as [`root`] drives it: `None` unless it is turn one (I start).
fn first_main(session: &mut Session) -> Option<(Pending, PlayerView)> {
    let (pending, view) = root(session, |view, _| view.turn <= 2);
    (view.turn == 1).then_some((pending, view))
}

/// Two tables that differ in what I cannot see (their hand, both
/// libraries, the seeds of the deal and of their house) and show me the
/// same root, with their presets and the root's question and view.
struct Twins {
    tables: Vec<Session>,
    presets: Vec<baylee_core::preset::GamePreset>,
    pending: Pending,
    view: PlayerView,
}

/// The first `wanted` twins among the deals of [`fixed_hand_preset`], each
/// paired with the first deal whose root was found.
fn twin_tables(wanted: usize) -> Vec<Twins> {
    let mut first: Option<(
        Session,
        baylee_core::preset::GamePreset,
        Pending,
        PlayerView,
    )> = None;
    let mut twins = Vec::new();
    for seed in 100..160 {
        let preset = fixed_hand_preset(seed);
        let mut session = seated(&preset);
        let Some((pending, view)) = first_main(&mut session) else {
            continue;
        };
        let Some((table, first_preset, first_pending, first_view)) = &first else {
            first = Some((session, preset, pending, view));
            continue;
        };
        if view != *first_view {
            continue;
        }
        assert_eq!(format!("{pending:?}"), format!("{first_pending:?}"));
        assert_ne!(
            session.snapshot_hash(),
            table.snapshot_hash(),
            "the twins differ in what I cannot see"
        );
        // Each pair owns its first table: replay it from its preset.
        let mut again = seated(first_preset);
        assert!(first_main(&mut again).is_some_and(|(_, v)| v == view));
        twins.push(Twins {
            tables: vec![again, session],
            presets: vec![first_preset.clone(), preset],
            pending,
            view,
        });
        if twins.len() == wanted {
            return twins;
        }
    }
    panic!("{} of {wanted} twin tables found", twins.len());
}

/// The search is a function of what I see: at twin tables it gives the
/// same report, the same answer and the same memory, through worlds the
/// house plays in.
#[test]
fn twin_tables_search_alike() {
    for twins in twin_tables(2) {
        let searched: Vec<_> = twins
            .tables
            .iter()
            .map(|table| {
                let mut player = Mock::new(steady, lands);
                player.observe(&twins.view).unwrap();
                let result = search_decision(
                    &mut player,
                    &KnownDecks(table),
                    &twins.view,
                    &twins.pending,
                    &[],
                    config(4, 3, 6),
                    SEED,
                )
                .unwrap();
                (result.answer.action, result.outcome, player.state)
            })
            .collect();
        let SearchOutcome4::Searched(report) = &searched[0].1 else {
            panic!("searched: {:?}", searched[0].1);
        };
        assert!(report.rollout_steps > 0, "the house played in the worlds");
        assert_eq!(searched[0], searched[1]);
    }
}

/// A table that forks `forks` worlds and refuses every one after.
struct ForksThenRefuses<'a> {
    session: &'a Session,
    forks: std::cell::Cell<usize>,
}
impl ForkSource4 for ForksThenRefuses<'_> {
    fn fork(&self, seat: PlayerId, seed: u64) -> Result<SimulationSession, ForkRefusal> {
        let left = self.forks.get();
        if left == 0 {
            return Err(ForkRefusal::FaceDown);
        }
        self.forks.set(left - 1);
        KnownDecks(self.session).fork(seat, seed)
    }
    fn decision_seq(&self) -> u64 {
        self.session.decision_seq()
    }
}

/// The worlds of a decision run at once on rollout players of the player's
/// own, at most `threads` together, and come to the same report, answer and
/// player as when they run one by one on the player itself; a table that
/// stops forking part way ends them the same way too.
#[test]
fn parallel_rounds_are_the_serial_rounds() {
    let mut session = table(25);
    let (pending, view) = root(&mut session, land_playable);
    let run = |lends: bool, threads: usize, forks: usize| {
        let mut player = Mock::new(steady, lands);
        if lends {
            player = player.lending();
        }
        player.observe(&view).unwrap();
        let table = ForksThenRefuses {
            session: &session,
            forks: std::cell::Cell::new(forks),
        };
        let result = search_decision(
            &mut player,
            &table,
            &view,
            &pending,
            &[],
            config(4, 3, 6).with_threads(threads).unwrap(),
            SEED,
        )
        .unwrap();
        (result.answer.action, result.outcome, player.state)
    };
    let (action, outcome, state) = run(false, 16, usize::MAX);
    let SearchOutcome4::Searched(report) = outcome else {
        panic!("searched: {outcome:?}");
    };
    assert!(
        !report.parallel,
        "a player that lends none rolls out serially"
    );
    assert_eq!(report.rollouts, 3 * report.candidates as u64);
    for threads in [2, 5, 12, 16] {
        let (parallel_action, parallel_outcome, parallel_state) = run(true, threads, usize::MAX);
        let SearchOutcome4::Searched(mut parallel) = parallel_outcome else {
            panic!("searched: {parallel_outcome:?}");
        };
        assert!(parallel.parallel, "{threads} threads");
        parallel.parallel = false;
        assert_eq!(parallel, report, "{threads} threads");
        assert_eq!(parallel_action, action);
        assert_eq!(
            parallel_state, state,
            "the player ends where answering would"
        );
    }
    let SearchOutcome4::Searched(one) = run(true, 1, usize::MAX).1 else {
        panic!("searched");
    };
    assert!(!one.parallel, "one thread is the serial path");
    // The second world's second fork is refused: the rollouts before it
    // run either way, and the policy answers, the refusal named.
    let forks = report.candidates + 1;
    let serial = run(false, 16, forks);
    assert_eq!(serial.1, SearchOutcome4::Refused(ForkRefusal::FaceDown));
    for threads in [2, 5, 16] {
        assert_eq!(run(true, threads, forks), serial, "{threads} threads");
    }
}

#[test]
fn a_refused_table_is_answered_by_the_policy_and_named() {
    let mut session = table(24);
    let (pending, view) = root(&mut session, any_root);
    let mut refused = Mock::new(steady, flat);
    let mut plain = Mock::new(steady, flat);
    let result = search_decision(
        &mut refused,
        &Refusing(&session),
        &view,
        &pending,
        &[],
        config(4, 2, 4),
        SEED,
    )
    .unwrap();
    let decision = plain.decide(&view, &pending, &[]);
    plain.conclude(&view, &decision, &decision.answer.action);
    assert_eq!(
        result.outcome,
        SearchOutcome4::Refused(ForkRefusal::FaceDown)
    );
    assert_eq!(result.answer.action, decision.answer.action);
    assert_eq!(result.answer.provenance.source, SourceKind4::Net);
    assert_eq!(refused.state, plain.state);
}

/// A whole game, every decision of seat 0 searched: every answer is legal
/// at the table, a search never moves the table, and the roots it does not
/// search are named.
#[test]
fn a_searched_game_answers_legally_and_names_every_root_it_skips() {
    let mut session = table(25);
    let mut player = Mock::new(steady, creatures);
    let mut counts: BTreeMap<String, u64> = BTreeMap::new();
    for _ in 0..3000 {
        if matches!(session.pending(), Pending::GameOver(_)) {
            break;
        }
        if session.needs_pump() {
            session.pump_at_most(1);
            continue;
        }
        let (pending, view) = session.view_for_with_context(ME).unwrap();
        player.observe(&view).unwrap();
        let hash = session.snapshot_hash();
        let result = search_decision(
            &mut player,
            &KnownDecks(&session),
            &view,
            &pending,
            &[],
            config(3, 1, 4),
            SEED,
        )
        .unwrap();
        assert_eq!(
            session.snapshot_hash(),
            hash,
            "a search never moves the table"
        );
        let key = match &result.outcome {
            SearchOutcome4::Searched(report) => {
                assert!(matches!(pending, Pending::Priority { .. }));
                assert_eq!(report.rounds, 1);
                "searched".to_owned()
            }
            SearchOutcome4::Skipped(reason) => {
                match reason {
                    SearchSkip4::NotPriority => {
                        assert!(!matches!(pending, Pending::Priority { .. }));
                    }
                    SearchSkip4::Phase => {
                        assert!(matches!(view.phase, Phase::Beginning | Phase::Ending));
                    }
                    SearchSkip4::NotPolicy | SearchSkip4::CandidateRefused => {}
                    SearchSkip4::Roots => panic!("no search here takes combat alone"),
                    SearchSkip4::MidChain | SearchSkip4::NoAlternative => {
                        panic!("no search here compares macro candidates")
                    }
                    SearchSkip4::TakenBack => panic!("no search here is told of a taking back"),
                }
                reason.key().to_owned()
            }
            SearchOutcome4::Refused(refusal) => format!("refused {refusal:?}"),
        };
        *counts.entry(key).or_default() += 1;
        session
            .act_slice(ME, result.answer.action, 0)
            .expect("every answer is legal at the table");
    }
    assert!(
        counts.get("searched").copied().unwrap_or(0) > 5,
        "{counts:?}"
    );
    assert!(counts.contains_key("not_priority"), "{counts:?}");
}

#[test]
fn budgets_parse_as_counts_within_bounds() {
    let parsed: SearchConfig4 = "4, 2, 0".parse().unwrap();
    assert_eq!(
        (parsed.candidates(), parsed.particles(), parsed.horizon()),
        (4, 2, 0)
    );
    for bad in [
        "0,1,1",
        "1,0,1",
        "4,2",
        "4,2,1,1",
        "a,2,1",
        "33,1,1",
        "1,1,10001",
    ] {
        assert!(bad.parse::<SearchConfig4>().is_err(), "{bad}");
    }
    assert_eq!(parsed.metadata()["contract"], SEARCH_CONTRACT);
    assert_eq!(parsed.selection(), Selection4::Prior { c: 1.0 });
    assert_eq!(Selection4::default(), Selection4::Prior { c: 1.0 });
    assert_eq!(parsed.metadata()["selection"]["rule"], "prior");
    assert_eq!(parsed.metadata()["default_selection"]["rule"], "prior");
    for bad in [
        Selection4::Paired {
            margin: -0.01,
            z: 1.0,
        },
        Selection4::Paired {
            margin: 0.05,
            z: f64::NAN,
        },
        Selection4::Prior { c: f64::INFINITY },
    ] {
        assert!(parsed.with_selection(bad).is_err(), "{bad:?}");
    }
    let paired = parsed.with_selection(Selection4::Mean).unwrap();
    assert_eq!(paired.metadata()["selection"]["rule"], "mean");

    assert_eq!(parsed.targets(), 0);
    assert!(parsed.metadata().get("targets").is_none());
    let targeted = parsed.with_targets(SearchConfig4::MAX_TARGETS).unwrap();
    assert_eq!(targeted.metadata()["contract"], SEARCH_CONTRACT_TARGETS);
    assert_eq!(targeted.metadata()["targets"], SearchConfig4::MAX_TARGETS);
    assert!(parsed.with_targets(SearchConfig4::MAX_TARGETS + 1).is_err());
}

/// A card of the catalog by name, as a deck entry.
fn entry(name: &str) -> baylee_core::preset::DeckEntry {
    baylee_core::preset::DeckEntry {
        card: baylee_cards::all()
            .find(|card| card.name() == name)
            .unwrap_or_else(|| panic!("no card {name}"))
            .index,
        print: baylee_core::ids::PrintRef::new(0),
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
    }
}

const SWORDS: &str = "Swords to Plowshares";

/// A table where I start, with Swords to Plowshares in hand and a Plains
/// and Grizzly Bears in play, and they have Savannah Lions; driven to my
/// first priority with Swords castable.
fn swords_table() -> (Session, Pending, PlayerView) {
    let mut preset = preset(31);
    preset.format = baylee_core::preset::FormatId::Freeform;
    preset.modifiers.clear();
    preset.house_rules.starting_player = Some(0);
    let hands: [&[&str]; 2] = [&[SWORDS], &[]];
    let boards: [&[&str]; 2] = [&["Plains", "Grizzly Bears"], &["Savannah Lions"]];
    for (index, seat) in preset.seats.iter_mut().enumerate() {
        seat.deck = vec![entry("Plains"); 32];
        seat.commanders.clear();
        seat.sideboard.clear();
        seat.starting_hand = Some(hands[index].iter().map(|name| entry(name)).collect());
        seat.starting_battlefield = boards[index].iter().map(|name| entry(name)).collect();
        seat.starting_life = Some(20);
    }
    let mut session = seated(&preset);
    let (pending, view) = root(&mut session, |view, pending| {
        let Pending::Priority { legal, .. } = pending else {
            return false;
        };
        view.hand.iter().any(|card| {
            card.name == SWORDS
                && (legal.castable.contains(&card.id) || legal.payable.contains(&card.id))
        })
    });
    (session, pending, view)
}

/// Casts Swords to Plowshares, at my own creature.
fn swords_at_my_own(view: &PlayerView, pending: &Pending) -> Option<PlayerAction> {
    match pending {
        Pending::Priority { .. } => view
            .hand
            .iter()
            .find(|card| card.name == SWORDS)
            .map(|card| PlayerAction::CastSpell { card: card.id }),
        Pending::ChooseTargets { options, .. } => options
            .iter()
            .find(|id| view.object(**id).is_some_and(|o| o.controller == view.seat))
            .map(|id| PlayerAction::ChooseTargets {
                objects: vec![*id],
                players: Vec::new(),
            }),
        _ => None,
    }
}

/// A critic that likes my creatures and dislikes theirs.
fn creature_balance(view: &PlayerView) -> f32 {
    let count = |mine: bool| {
        view.battlefield
            .iter()
            .filter(|o| (o.controller == view.seat) == mine && o.types.contains(TypeSet::CREATURE))
            .count()
    };
    let difference = i16::try_from(count(true)).unwrap() - i16::try_from(count(false)).unwrap();
    0.5 + 0.1 * f32::from(difference)
}

/// The object of `view` named `name`.
fn named(view: &PlayerView, name: &str) -> baylee_core::ids::ObjectId {
    view.battlefield
        .iter()
        .find(|o| o.name == name)
        .unwrap_or_else(|| panic!("no {name}"))
        .id
}

/// With targets searched, a policy that would exile its own creature is
/// tried at each target, and the search chooses theirs: the cast is
/// played, and the chosen target answers the target question at the table
/// as a searched answer.
#[test]
fn a_cast_is_tried_at_each_target_and_the_search_chooses_theirs() {
    let (mut table, pending, view) = swords_table();
    let mine = named(&view, "Grizzly Bears");
    let theirs = named(&view, "Savannah Lions");
    let hash = table.snapshot_hash();
    let mut player = Mock::new(swords_at_my_own, creature_balance);
    player.observe(&view).unwrap();
    let result = search_decision(
        &mut player,
        &KnownDecks(&table),
        &view,
        &pending,
        &[],
        config(1, 2, 4).with_targets(4).unwrap(),
        SEED,
    )
    .unwrap();
    let SearchOutcome4::Searched(report) = &result.outcome else {
        panic!("the root is searched: {:?}", result.outcome);
    };
    let at = |id| {
        Some(PlayerAction::ChooseTargets {
            objects: vec![id],
            players: Vec::new(),
        })
    };
    let cast = swords_at_my_own(&view, &pending).unwrap();
    assert_eq!(report.actions, [cast.clone(), cast.clone()]);
    assert_eq!(
        report.targets,
        [at(mine), at(theirs)],
        "the policy's own first"
    );
    assert_eq!((report.rounds, report.chosen), (2, 1), "{report:?}");
    assert!(report.changed);
    assert!(report.means[1] > report.means[0], "{:?}", report.means);
    assert_eq!(result.answer.action, cast);
    assert_eq!(result.answer.provenance.source, SourceKind4::Search);
    assert_eq!(result.follow, at(theirs));
    assert_eq!(table.snapshot_hash(), hash, "no world touched the table");

    // At the table: the cast, then its target question, answered with the
    // chosen target as a searched answer.
    table.act_slice(ME, cast, 0).unwrap();
    while table.needs_pump() {
        table.pump_at_most(1);
    }
    let (question, asked) = table.view_for_with_context(ME).unwrap();
    assert!(
        matches!(question, Pending::ChooseTargets { .. }),
        "{question:?}"
    );
    player
        .observe(&table.observe_view_with_context(ME))
        .unwrap();
    let follow = result.follow.unwrap();
    let (answer, followed) = follow_decision(&mut player, &asked, &question, &[], &follow);
    assert!(followed);
    assert_eq!(answer.action, follow);
    assert_eq!(answer.provenance.source, SourceKind4::Search);
    let kind = PendingKind4::of(&question) as usize;
    assert_eq!(player.state.coverage[kind][SourceKind4::Search as usize], 1);
    table.act_slice(ME, answer.action, 0).unwrap();

    // A target the question does not offer is the policy's to choose.
    let (mut again, pending, view) = swords_table();
    let cast = swords_at_my_own(&view, &pending).unwrap();
    again.act_slice(ME, cast, 0).unwrap();
    while again.needs_pump() {
        again.pump_at_most(1);
    }
    let (question, asked) = again.view_for_with_context(ME).unwrap();
    let mut plain = Mock::new(swords_at_my_own, creature_balance);
    plain.observe(&again.observe_view_with_context(ME)).unwrap();
    let elsewhere = PlayerAction::ChooseTargets {
        objects: Vec::new(),
        players: vec![ME],
    };
    let (answer, followed) = follow_decision(&mut plain, &asked, &question, &[], &elsewhere);
    assert!(!followed);
    assert_eq!(Some(answer.action), at(mine));
}

/// Without targets searched (v2), the cast is one candidate and its target
/// stays the policy's.
#[test]
fn without_targets_a_cast_is_one_candidate() {
    let (table, pending, view) = swords_table();
    let mut player = Mock::new(swords_at_my_own, creature_balance);
    player.observe(&view).unwrap();
    let result = search_decision(
        &mut player,
        &KnownDecks(&table),
        &view,
        &pending,
        &[],
        config(1, 2, 4),
        SEED,
    )
    .unwrap();
    let SearchOutcome4::Searched(report) = &result.outcome else {
        panic!("the root is searched: {:?}", result.outcome);
    };
    assert_eq!(report.candidates, 1);
    assert_eq!(report.targets, [None]);
    assert_eq!(result.follow, None);
}

/// Drives the table, seat 0 by the steady house, until seat 0 is asked to
/// declare attackers with two creatures able to attack, past the opening
/// turns, and the table forks there.
fn attack_root(session: &mut Session) -> (Pending, PlayerView) {
    for _ in 0..5000 {
        assert!(!matches!(session.pending(), Pending::GameOver(_)));
        if session.needs_pump() {
            session.pump_at_most(1);
            continue;
        }
        let (pending, view) = session.view_for_with_context(ME).unwrap();
        if matches!(&pending, Pending::ChooseAttackers { attackers, .. } if attackers.len() >= 2)
            && KnownDecks(session).fork(ME, 1).is_ok()
        {
            return (pending, view);
        }
        let action = session.house_action(ME).unwrap();
        session.act_slice(ME, action, 0).unwrap();
    }
    panic!("no declaration of attackers to search");
}

/// Declares no attackers and blocks with nothing.
#[allow(
    clippy::unnecessary_wraps,
    reason = "One signature for every stand-in policy"
)]
fn holds_back(_: &PlayerView, pending: &Pending) -> Option<PlayerAction> {
    match pending {
        Pending::ChooseAttackers { .. } => {
            Some(PlayerAction::DeclareAttackers { attackers: vec![] })
        }
        Pending::ChooseBlockers { .. } => Some(PlayerAction::DeclareBlockers { blockers: vec![] }),
        _ => None,
    }
}

/// A critic that likes the other side's life lost.
fn their_life_lost(view: &PlayerView) -> f32 {
    let lowest = view
        .seats
        .iter()
        .filter(|seat| seat.player != view.seat)
        .map(|seat| seat.life)
        .min()
        .unwrap_or(20);
    0.5 + f32::from(u8::try_from((20 - lowest).clamp(0, 20)).unwrap_or(0)) / 40.0
}

/// With combat roots on, a declaration of attackers is searched: the
/// policy's declaration first, every candidate a declaration the engine
/// takes, and the answer one of them, recorded as the search's. Without
/// them the declaration is the policy's, as before.
#[test]
fn a_declaration_of_attackers_is_searched_with_combat_roots() {
    let mut table = table(5);
    let (pending, view) = attack_root(&mut table);
    let mut player = Mock::new(holds_back, their_life_lost);
    player.observe(&view).unwrap();
    let before = player.snapshot();
    let plain = search_decision(
        &mut player,
        &KnownDecks(&table),
        &view,
        &pending,
        &[],
        config(4, 2, 4),
        SEED,
    )
    .unwrap();
    assert!(
        matches!(
            plain.outcome,
            SearchOutcome4::Skipped(SearchSkip4::NotPriority)
        ),
        "{:?}",
        plain.outcome
    );
    player.restore(&before);
    let combat = config(4, 2, 4).with_combat(6).unwrap();
    assert_eq!(combat.contract(), SEARCH_CONTRACT_COMBAT);
    let result = search_decision(
        &mut player,
        &KnownDecks(&table),
        &view,
        &pending,
        &[],
        combat,
        SEED,
    )
    .unwrap();
    let SearchOutcome4::Searched(report) = &result.outcome else {
        panic!("the declaration is searched: {:?}", result.outcome);
    };
    assert!(report.candidates >= 2, "{report:?}");
    assert_eq!(
        report.actions[0],
        PlayerAction::DeclareAttackers { attackers: vec![] },
        "the policy's declaration first"
    );
    assert!(
        report
            .actions
            .iter()
            .all(|action| matches!(action, PlayerAction::DeclareAttackers { .. }))
    );
    assert!((report.priors[0] - 1.0).abs() < f64::EPSILON);
    assert!(report.priors[1..].iter().all(|prior| *prior == 0.0));
    assert!(report.actions.contains(&result.answer.action));
    assert_eq!(result.answer.provenance.source, SourceKind4::Search);
    assert!(pending.answer_fault(&result.answer.action).is_none());
}

/// A search for attacks and blocks alone leaves a priority decision to the
/// policy, and says why.
#[test]
fn combat_only_leaves_priority_to_the_policy() {
    let mut table = table(5);
    let (pending, view) = root(&mut table, any_root);
    let mut player = Mock::new(steady, flat);
    player.observe(&view).unwrap();
    let config = config(4, 2, 4)
        .with_combat(6)
        .unwrap()
        .with_combat_only(true);
    let result = search_decision(
        &mut player,
        &KnownDecks(&table),
        &view,
        &pending,
        &[],
        config,
        SEED,
    )
    .unwrap();
    assert!(
        matches!(result.outcome, SearchOutcome4::Skipped(SearchSkip4::Roots)),
        "{:?}",
        result.outcome
    );
    assert_eq!(
        config.metadata()["roots"],
        "declarations_of_attackers_and_blockers_only"
    );
}

#[test]
fn seeds_are_tagged_by_purpose_and_position() {
    let base = search_seed(7);
    assert_ne!(base, search_seed(8));
    assert_ne!(world_seed(base, 10, 0), world_seed(base, 10, 1));
    assert_ne!(world_seed(base, 10, 0), world_seed(base, 11, 0));
    assert_ne!(derive(base, WORLD, 1, 0), derive(base, ROLLOUT, 1, 0));
    assert_eq!(world_seed(base, 10, 1), world_seed(base, 10, 1));
}

/// Seat 0 of a real table, played by `searched` searching under `config`
/// and by `plain` beside it, until three roots are searched: a
/// one-candidate search answers what the net answers, and every world is
/// undone down to the last float of the player's memory. The reports.
fn searched_beside_plain(
    mut searched: NetPlayer4,
    mut plain: NetPlayer4,
    config: SearchConfig4,
) -> Vec<SearchReport4> {
    assert_eq!(config.candidates(), 1);
    let mut reports = Vec::new();
    let mut session = table(26);
    searched.init_game(&preset(26));
    plain.init_game(&preset(26));
    for _ in 0..400 {
        if matches!(session.pending(), Pending::GameOver(_)) || reports.len() == 3 {
            break;
        }
        if session.needs_pump() {
            session.pump_at_most(1);
            continue;
        }
        let observed = session.observe_view_with_context(ME);
        searched.observe(&observed).unwrap();
        plain.observe(&observed).unwrap();
        let (pending, view) = session.view_for_with_context(ME).unwrap();
        let result = search_decision(
            &mut searched,
            &KnownDecks(&session),
            &view,
            &pending,
            &[],
            config,
            SEED,
        )
        .unwrap();
        let answer = plain.act_detailed(&view, &pending, &[]);
        assert_eq!(result.answer.action, answer.action);
        if let SearchOutcome4::Searched(report) = result.outcome {
            assert_eq!(
                report.leaves.terminal + report.leaves.value(),
                report.rollouts
            );
            SearchPlayer4::attribute_to_search(&mut plain, &pending, answer.provenance.source);
            reports.push(*report);
        }
        assert_eq!(
            format!("{:?}", searched.snapshot()),
            format!("{:?}", plain.snapshot()),
            "every world undone"
        );
        session.act_slice(ME, answer.action, 0).unwrap();
    }
    assert!(!reports.is_empty(), "a root was searched");
    reports
}

/// The real net, on the CPU, on sessions of its own: the rollouts run one
/// by one on the player. Run with `BAYLEE_TEST_MODEL=DIR BAYLEE_EP=cpu`
/// and `--ignored`.
#[test]
#[ignore = "needs BAYLEE_TEST_MODEL, an exported v4/v5 model directory"]
fn a_real_net_ends_a_search_where_answering_would_have_left_it() {
    let Ok(dir) = std::env::var("BAYLEE_TEST_MODEL") else {
        return;
    };
    // Undo does not depend on the catalog: an older export binds as a superset.
    let load = || {
        NetPlayer4::load_with(
            std::path::Path::new(&dir),
            crate::netplay4::CatalogPolicy4::Drift,
        )
        .unwrap()
    };
    let reports = searched_beside_plain(load(), load(), config(1, 2, 8));
    assert!(reports.iter().all(|r| !r.parallel && r.rounds == 2));
}

/// The real net, on the CPU, on one batched session: the rollouts run at
/// once on twins whose steps share its batches, while the searching
/// player's own steps run alone, so it still ends where answering would
/// have left it, bit for bit. Run with `BAYLEE_TEST_MODEL=DIR
/// BAYLEE_EP=cpu` and `--ignored`.
#[test]
#[ignore = "needs BAYLEE_TEST_MODEL, an exported v4/v5 model directory"]
fn a_real_net_on_a_batched_session_searches_its_worlds_at_once() {
    let Ok(dir) = std::env::var("BAYLEE_TEST_MODEL") else {
        return;
    };
    let net = crate::netplay4::BatchedNet4::load(
        std::path::Path::new(&dir),
        crate::netplay4::BatchLimits4::for_rows(16),
        crate::netplay4::CatalogPolicy4::Drift,
    )
    .unwrap();
    let searched = NetPlayer4::shared(&net);
    assert!(SearchPlayer4::lends_rollout_players(&searched));
    let reports = searched_beside_plain(
        searched,
        NetPlayer4::shared(&net),
        config(1, 4, 6).with_threads(4).unwrap(),
    );
    assert!(reports.iter().all(|r| r.parallel && r.rounds == 4));
    let (calls, rows) = net.stats();
    assert!(
        rows > calls,
        "the rollouts shared batches: {rows} rows in {calls} calls"
    );
}

/// The real net, on the CPU, at twin tables: its memory starts from each
/// table's own preset and takes in the same root, and the search gives the
/// same report and answer and leaves the same memory, bit for bit. Run with
/// `BAYLEE_TEST_MODEL=DIR BAYLEE_EP=cpu` and `--ignored`.
#[test]
#[ignore = "needs BAYLEE_TEST_MODEL, an exported v4/v5 model directory"]
fn a_real_net_searches_twin_tables_alike() {
    let Ok(dir) = std::env::var("BAYLEE_TEST_MODEL") else {
        return;
    };
    let twins = twin_tables(1).remove(0);
    let searched: Vec<_> = twins
        .tables
        .iter()
        .zip(&twins.presets)
        .map(|(table, preset)| {
            let mut net = NetPlayer4::load_with(
                std::path::Path::new(&dir),
                crate::netplay4::CatalogPolicy4::Drift,
            )
            .unwrap();
            net.init_game(preset);
            net.observe(&twins.view).unwrap();
            let result = search_decision(
                &mut net,
                &KnownDecks(table),
                &twins.view,
                &twins.pending,
                &[],
                config(4, 2, 6),
                SEED,
            )
            .unwrap();
            let memory = format!("{:?}", net.snapshot());
            (result.answer.action, result.outcome, memory)
        })
        .collect();
    assert!(matches!(searched[0].1, SearchOutcome4::Searched(_)));
    assert_eq!(searched[0], searched[1]);
}
