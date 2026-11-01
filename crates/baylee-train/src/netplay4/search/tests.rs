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
/// the real encoder's menu, in menu order, its own answer's logit raised by
/// `confidence`.
struct Mock {
    state: MockState,
    own: fn(&PlayerView, &Pending) -> Option<PlayerAction>,
    value: fn(&PlayerView) -> f32,
    confidence: f32,
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
            confidence: 0.0,
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
        let action = (self.own)(view, pending)
            .filter(|a| pending.answer_fault(a).is_none())
            .unwrap_or_else(|| HeuristicAgent::new(AIProfile::STEADY).act(view, pending));
        let source = if policy {
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

/// Victory against Victory.
fn preset(seed: u64) -> baylee_core::preset::GamePreset {
    let deck = crate::housedeck::HouseDeck::named("victory").unwrap();
    crate::selfplay::table_for(seed, &[&deck, &deck], &[AIProfile::STEADY; 2], &[])
}

/// [`preset`], seat 0 taken over, recorded.
fn table(seed: u64) -> Session {
    let mut session = Session::new_recorded(&preset(seed), "test").unwrap();
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
        !changed(Selection4::default()),
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
    assert_eq!(parsed.selection(), Selection4::default());
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

/// The real net, on the CPU: a one-candidate search answers what the net
/// answers, and every world is undone down to the last float of its
/// memory. Run with `BAYLEE_TEST_MODEL=DIR BAYLEE_EP=cpu` and `--ignored`.
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
    let (mut searched, mut plain) = (load(), load());
    let mut session = table(26);
    searched.init_game(&preset(26));
    plain.init_game(&preset(26));
    let mut roots = 0;
    for _ in 0..400 {
        if matches!(session.pending(), Pending::GameOver(_)) || roots == 3 {
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
            config(1, 2, 8),
            SEED,
        )
        .unwrap();
        let answer = plain.act_detailed(&view, &pending, &[]);
        assert_eq!(result.answer.action, answer.action);
        if let SearchOutcome4::Searched(report) = &result.outcome {
            roots += 1;
            assert_eq!(
                report.leaves.terminal + report.leaves.value(),
                report.rollouts
            );
            SearchPlayer4::attribute_to_search(&mut plain, &pending, answer.provenance.source);
        }
        assert_eq!(
            format!("{:?}", searched.snapshot()),
            format!("{:?}", plain.snapshot()),
            "every world undone"
        );
        session.act_slice(ME, answer.action, 0).unwrap();
    }
    assert!(roots > 0, "a root was searched");
}
