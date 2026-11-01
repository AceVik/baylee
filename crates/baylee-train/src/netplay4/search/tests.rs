//! The search on real tables with a stand-in for the net: the plumbing,
//! not playing strength. No model is loaded.
use std::collections::BTreeMap;

use super::*;
use crate::features4::{CoverageMatrix4, HeadKind4, PendingKind4};
use crate::netplay4::rank_single_choice;
use baylee_ai::HeuristicAgent;
use baylee_core::preset::AIProfile;
use baylee_core::types::TypeSet;

const ME: PlayerId = PlayerId::new(0);
const SEED: u64 = 0x0b5e_55ed_5eed_0001;

/// What the stand-in changes while it plays, all of it.
#[derive(Clone, Debug, Default, PartialEq)]
struct MockState {
    observed: BTreeMap<u8, f32>,
    observations: u64,
    answers: u64,
    concluded: Vec<PlayerAction>,
    sampling: u64,
    coverage: Vec<[u64; 5]>,
}

/// A stand-in for the net: its own answer at a root is `own`, its critic is
/// `value`, its rollout answers are the steady house's, and its ranking is
/// the real encoder's menu, in menu order.
struct Mock {
    state: MockState,
    own: fn(&PlayerView, &Pending) -> Option<PlayerAction>,
    value: fn(&PlayerView) -> f32,
}

impl Mock {
    fn new(
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
        let action = (self.own)(view, pending)
            .filter(|a| pending.answer_fault(a).is_none())
            .unwrap_or_else(|| HeuristicAgent::new(AIProfile::STEADY).act(view, pending));
        let source = if policy {
            SourceKind4::Net
        } else {
            SourceKind4::Forced
        };
        self.record(pending, source);
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
            ranking: policy.then(|| {
                rank_single_choice(&cands, &vec![0.0; cands.candidates.len()], view, pending)
            }),
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
        Answer4 {
            action,
            value: 0.5,
            provenance: StepProvenance4::default(),
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
}

/// Passes whenever it may.
fn passes(_: &PlayerView, pending: &Pending) -> Option<PlayerAction> {
    matches!(pending, Pending::Priority { .. }).then_some(PlayerAction::PassPriority)
}

/// Plays as the steady house.
#[allow(
    clippy::unnecessary_wraps,
    reason = "One signature for every stand-in policy"
)]
fn steady(view: &PlayerView, pending: &Pending) -> Option<PlayerAction> {
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
fn creatures(view: &PlayerView) -> f32 {
    counting(view, TypeSet::CREATURE)
}

/// A critic that likes the seat's lands: the obvious answer to a land in
/// hand is to play it.
fn lands(view: &PlayerView) -> f32 {
    counting(view, TypeSet::LAND)
}

/// Victory against Victory, seat 0 taken over, recorded.
fn table(seed: u64) -> Session {
    let deck = crate::housedeck::HouseDeck::named("victory").unwrap();
    let preset = crate::selfplay::table_for(seed, &[&deck, &deck], &[AIProfile::STEADY; 2], &[]);
    let mut session = Session::new_recorded(&preset, "test").unwrap();
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

fn config(k: usize, p: usize, h: usize) -> SearchConfig4 {
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
    assert!(!report.changed);
    assert_eq!(report.rollouts, 3);
    assert!(report.rollout_steps > 0, "the worlds were played on");
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

#[test]
fn a_value_lookahead_finds_the_land_drop_the_policy_passed_on() {
    let mut session = table(22);
    let (pending, view) = root(&mut session, land_playable);
    let mut player = Mock::new(passes, lands);
    player.observe(&view).unwrap();
    let result = search_decision(
        &mut player,
        &KnownDecks(&session),
        &view,
        &pending,
        &[],
        config(SearchConfig4::MAX_CANDIDATES, 1, 0),
        SEED,
    )
    .unwrap();
    let SearchOutcome4::Searched(report) = &result.outcome else {
        panic!("the root is searched: {:?}", result.outcome);
    };
    assert!(report.changed, "{report:?}");
    assert_eq!(report.value_leaves, report.rollouts);
    assert_eq!(
        report.rollout_steps, 0,
        "H = 0 scores the answer's own position"
    );
    assert!(report.means[report.chosen] > report.means[0]);
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
    session
        .act_slice(ME, result.answer.action, 0)
        .expect("the searched answer is legal at the table");
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
}

#[test]
fn ties_go_to_the_policy_then_to_candidate_order() {
    assert_eq!(best(&[0.5, 0.5, 0.4]), 0);
    assert_eq!(best(&[0.4, 0.6, 0.6]), 1);
    assert_eq!(best(&[0.4]), 0);
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
