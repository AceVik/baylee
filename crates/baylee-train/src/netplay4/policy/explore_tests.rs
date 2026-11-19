//! Rule proposals as recorded exploration (`netplay4::explore`): the mixture
//! probability is exact, a run replays bit for bit from its seeds, and an
//! explorer never moves the net's own draw.
use super::*;
use crate::netplay4::RuleExplorer4;
use baylee_core::preset::AIProfile;
use baylee_engine::choice::{AttackerBound, BlockOption, NumberPrompt};
use baylee_train_ops::exploration::{
    self, NET, RuleProposal, RunExploration, StepExploration, check_step,
};
use baylee_train_ops::measured::RULE_GROUPS;
use std::collections::BTreeMap;

const TAU: f32 = 0.7;
const EPSILON: f64 = 0.3;

fn view() -> PlayerView {
    let deck = crate::housedeck::HouseDeck::named("victory").unwrap();
    let mut preset = crate::selfplay::table_for(12, &[&deck, &deck], &[AIProfile::STEADY; 2], &[]);
    preset.format = baylee_core::preset::FormatId::Freeform;
    preset.modifiers.clear();
    let spell = baylee_cards::all()
        .find(|card| card.name() == "Shock")
        .unwrap()
        .index;
    for seat in &mut preset.seats {
        let mut entry = seat.deck[0];
        entry.card = spell;
        entry.rule_print = None;
        entry.choices = baylee_core::card_metadata::DeckChoices::default();
        seat.starting_hand = Some(vec![entry; 3]);
    }
    baylee_gamehost::Session::new(&preset)
        .unwrap()
        .observe_view(PlayerId::new(0))
}

fn menu(view: &PlayerView, pending: &Pending) -> EncodedCandidates4 {
    let state = encode_state4(view, Some(pending), None, &[]);
    encode_candidates4(&state, view, pending)
}

fn explorer() -> RuleExplorer4 {
    RuleExplorer4::new(TacticRules4::parse(RULE_GROUPS).unwrap(), EPSILON).unwrap()
}

fn proposers() -> Vec<String> {
    let names: Vec<&str> = RULE_GROUPS.split(',').collect();
    RunExploration::new(EPSILON, RULE_GROUPS, &names).proposers
}

/// One raw decision at `TAU`, the net's stream seeded `seed`; with an
/// explorer its record. Returns the target, the behaviour log-probability
/// (`None` where the answer has none) and the net's stream afterwards.
fn decide(
    explorer: Option<&mut RuleExplorer4>,
    seed: u64,
    view: &PlayerView,
    pending: &Pending,
    cands: &EncodedCandidates4,
    logits: &[f32],
    scalar: f32,
) -> (TeacherTarget4, Option<f32>, u64) {
    let mut rng = Rng::new(seed);
    let (_, _, lp, target) = PolicySampler4 {
        tactician: None,
        explorer,
        mode: PolicyMode4::Raw,
        temperature: TAU,
        rng: &mut rng,
        active_source: None,
        guard: &mut crate::netplay4::ChoiceGuard4::default(),
        rules: &mut TacticRules4::default(),
    }
    .select(
        cands,
        view,
        pending,
        PolicyOutputs4 {
            embeddings: None,
            cand_logits: logits,
            pair_logits: logits,
            scalar_pred: scalar,
            value: 0.4,
        },
    )
    .unwrap();
    (target, lp, rng.state())
}

/// What identifies an answer of any head.
fn key(target: &TeacherTarget4) -> Key {
    (
        target.single_idx,
        target.multi_mask.clone(),
        target.scalar_norm.to_bits(),
    )
}

/// What identifies an answer of any head: index, mask, number.
type Key = (u16, Vec<u8>, u32);
/// Per answer: π and μ as bits (`None` where there is none), and how often
/// it was played.
type Answers = BTreeMap<Key, (Option<u32>, Option<u32>, u64)>;
type Record = (Key, Option<u32>, StepExploration);

/// `draws` explored decisions, each on its own net seed and game seed.
fn explore(
    view: &PlayerView,
    pending: &Pending,
    cands: &EncodedCandidates4,
    logits: &[f32],
    scalar: f32,
    draws: u64,
) -> Vec<Record> {
    let mut explorer = explorer();
    (0..draws)
        .map(|i| {
            explorer.reseed(9_000 + i);
            let (target, lp, _) = decide(
                Some(&mut explorer),
                1_000 + i,
                view,
                pending,
                cands,
                logits,
                scalar,
            );
            let record = explorer.take_last().expect("an explored decision");
            (key(&target), lp.map(f32::to_bits), record)
        })
        .collect()
}

/// Every record checks against its own π and ε; each answer has one π and
/// one μ, whether the net sampled it or the layer played it; the answers'
/// frequencies are their μ; and a second run is the same run.
fn assert_exact(
    view: &PlayerView,
    pending: &Pending,
    cands: &EncodedCandidates4,
    logits: &[f32],
    scalar: f32,
    stochastic: bool,
) -> Answers {
    let draws = 6_000;
    let records = explore(view, pending, cands, logits, scalar, draws);
    assert_eq!(
        records,
        explore(view, pending, cands, logits, scalar, draws),
        "a run replays from its seeds"
    );
    assert!(
        records.iter().any(|(_, _, record)| record.played)
            && records.iter().any(|(_, _, record)| !record.played),
        "the fixture plays both the layer's answers and the net's"
    );
    let table = proposers();
    let mut answers = Answers::new();
    for (answer, lp, record) in &records {
        assert_eq!(lp.is_some(), stochastic, "a sampled head has its π");
        let behavior_logprob = lp.map_or(0.0, f32::from_bits);
        check_step(record, EPSILON, stochastic, behavior_logprob, &table).unwrap();
        assert_eq!(record.draw, 0, "one draw per decision of a game");
        let mixture = record.mixture_logprob.map(f32::to_bits);
        assert!(mixture.is_some(), "every answer here has its μ");
        let entry = answers.entry(answer.clone()).or_insert((*lp, mixture, 0));
        assert_eq!(entry.0, *lp, "π(a) is the answer's, however it was chosen");
        assert_eq!(entry.1, mixture, "μ(a) too");
        entry.2 += 1;
    }
    let mut total = 0.0;
    for (answer, &(_, mixture, count)) in &answers {
        let mu = f64::from(f32::from_bits(mixture.expect("checked above"))).exp();
        total += mu;
        #[allow(clippy::cast_precision_loss, reason = "a few thousand draws")]
        let frequency = count as f64 / draws as f64;
        let sigma = (mu * (1.0 - mu) / 6_000.0).sqrt();
        assert!(
            (frequency - mu).abs() <= 4.5 * sigma + 1e-3,
            "{answer:?}: frequency {frequency} against μ {mu}"
        );
    }
    assert!(total <= 1.0 + 1e-5, "Σ μ = {total}");
    answers
}

#[test]
fn single_choices_mix_exactly_and_replay() {
    let view = view();
    // A hand of Shocks and no land: the layer sends it back, the net keeps it.
    let pending = Pending::Mulligan {
        player: view.seat,
        taken: 0,
        next_is_free: false,
        can_take: true,
    };
    let cands = menu(&view, &pending);
    let logits = [1.5_f32, -0.5];
    let answers = assert_exact(&view, &pending, &cands, &logits, 0.2, true);
    assert_eq!(answers.len(), 2, "both answers are played");
    let total: f64 = answers
        .values()
        .filter_map(|&(_, mixture, _)| mixture)
        .map(|mixture| f64::from(f32::from_bits(mixture)).exp())
        .sum();
    assert!((total - 1.0).abs() < 1e-5, "Σ μ = {total}");
    let records = explore(&view, &pending, &cands, &logits, 0.2, 200);
    assert!(records.iter().any(|(_, _, r)| r.played));
    assert!(
        records
            .iter()
            .any(|(_, _, r)| r.rule == RuleProposal::Differs && !r.played)
    );
    assert!(records.iter().any(|(_, _, r)| r.rule == RuleProposal::Same));
    assert!(
        records
            .iter()
            .filter(|(_, _, r)| r.played)
            .all(|(_, _, r)| r.proposer != NET),
        "a played layer answer names its proposer"
    );
}

#[test]
fn subsets_and_pairs_mix_exactly_and_replay() {
    let view = view();
    let subset = Pending::DiscardChoice {
        player: view.seat,
        count: 1,
    };
    let subset_menu = menu(&view, &subset);
    let logits: Vec<f32> = (0..subset_menu.candidates.len())
        .map(|i| [0.9, -0.3, 0.2][i % 3])
        .collect();
    assert_exact(&view, &subset, &subset_menu, &logits, 0.2, true);

    let (view, attack) = combat(false);
    let attack_menu = menu(&view, &attack);
    let logits: Vec<f32> = (0..attack_menu.candidates.len())
        .map(|i| [0.3, -0.6, 0.1][i % 3])
        .collect();
    assert_exact(&view, &attack, &attack_menu, &logits, 0.2, true);

    let (view, block) = combat(true);
    let block_menu = menu(&view, &block);
    assert!(!block_menu.pairs.is_empty());
    let logits: Vec<f32> = (0..block_menu.pairs.len())
        .map(|i| [0.2, -0.5, 0.6][i % 3])
        .collect();
    assert_exact(&view, &block, &block_menu, &logits, 0.2, true);
}

/// Two Grizzly Bears a side: seat 0 declaring its attack, or (`block`)
/// declaring its blockers against seat 1's bears.
fn combat(block: bool) -> (PlayerView, Pending) {
    use baylee_core::ids::Defender;
    let deck = crate::housedeck::HouseDeck::named("victory").unwrap();
    let mut preset = crate::selfplay::table_for(31, &[&deck, &deck], &[AIProfile::STEADY; 2], &[]);
    preset.format = baylee_core::preset::FormatId::Freeform;
    preset.modifiers.clear();
    let mut bears = preset.seats[0].deck[0];
    bears.card = baylee_cards::decks::by_name("Grizzly Bears").unwrap();
    bears.rule_print = None;
    bears.choices = baylee_core::card_metadata::DeckChoices::default();
    for seat in &mut preset.seats {
        seat.commanders.clear();
        seat.starting_hand = Some(vec![]);
        seat.starting_battlefield = vec![bears; 2];
    }
    let me = PlayerId::new(0);
    let mut session = baylee_gamehost::Session::new(&preset).unwrap();
    for seat in 0..2 {
        session.take_over(PlayerId::new(seat));
    }
    for _ in 0..512 {
        let pending = session.pending().clone();
        let asked = pending.asked().unwrap();
        let reached = match &pending {
            Pending::ChooseAttackers { attackers, .. } => {
                !block && asked == me && attackers.len() == 2
            }
            Pending::ChooseBlockers { .. } => block && asked == me,
            _ => false,
        };
        if reached {
            let (pending, view) = session.view_for_with_context(me).unwrap();
            return (view, pending);
        }
        let action = match &pending {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::ChooseAttackers { attackers, .. } if block && asked != me => {
                PlayerAction::DeclareAttackers {
                    attackers: attackers
                        .iter()
                        .map(|&id| (id, Defender::Player(me)))
                        .collect(),
                }
            }
            Pending::ChooseAttackers { .. } => PlayerAction::DeclareAttackers {
                attackers: Vec::new(),
            },
            Pending::ChooseBlockers { .. } => PlayerAction::DeclareBlockers {
                blockers: Vec::new(),
            },
            Pending::Priority { .. } => PlayerAction::PassPriority,
            _ => session.house_action(asked).unwrap(),
        };
        session.act(asked, action).unwrap();
    }
    panic!("combat not reached");
}

#[test]
fn a_number_is_a_point_mass_and_the_layer_plays_at_rate_epsilon() {
    let view = view();
    let pending = Pending::ChooseNumber {
        player: view.seat,
        min: 0,
        max: 10,
        reason: NumberPrompt::X,
    };
    let mut cands = menu(&view, &pending);
    cands.min_choices = 0;
    cands.max_choices = 10;
    // The net says 2, the layer all of X.
    let answers = assert_exact(&view, &pending, &cands, &[], 0.2, false);
    assert_eq!(answers.len(), 2);
    let mixtures: Vec<f64> = answers
        .values()
        .map(|&(lp, mixture, _)| {
            assert_eq!(lp, None, "a number claims no probability");
            f64::from(f32::from_bits(mixture.expect("a point mass's μ"))).exp()
        })
        .collect();
    assert!(mixtures.iter().any(|mu| (mu - EPSILON).abs() < 1e-6));
    assert!(
        mixtures
            .iter()
            .any(|mu| (mu - (1.0 - EPSILON)).abs() < 1e-6)
    );
}

/// The layer's answer is read on copies: with an explorer the net draws what
/// it draws without one, and its stream ends where it would.
#[test]
fn an_explorer_never_moves_the_nets_own_draw() {
    let view = view();
    let pending = Pending::Mulligan {
        player: view.seat,
        taken: 0,
        next_is_free: false,
        can_take: true,
    };
    let cands = menu(&view, &pending);
    let logits = [1.5_f32, -0.5];
    let mut explorer = explorer();
    for seed in 0..64 {
        explorer.reseed(seed);
        let alone = decide(None, seed, &view, &pending, &cands, &logits, 0.2);
        let explored = decide(
            Some(&mut explorer),
            seed,
            &view,
            &pending,
            &cands,
            &logits,
            0.2,
        );
        assert_eq!(alone.2, explored.2, "the net's stream");
        let record = explorer.take_last().unwrap();
        if !record.played {
            assert_eq!((key(&alone.0), alone.1), (key(&explored.0), explored.1));
        }
    }
}

/// Two blockers, `b0` and `b1`, that may each block `a0` or `a1`; `a0` has
/// menace (CR 702.111b) and states that it needs two.
fn menace_block(view: &PlayerView) -> (Pending, EncodedCandidates4) {
    let (b0, b1) = (ObjectId::new(20, 0), ObjectId::new(21, 0));
    let (a0, a1) = (ObjectId::new(30, 0), ObjectId::new(31, 0));
    let pending = Pending::ChooseBlockers {
        player: view.seat,
        attacker: PlayerId::new(1),
        blockers: vec![
            BlockOption {
                blocker: b0,
                attackers: vec![a0, a1],
            },
            BlockOption {
                blocker: b1,
                attackers: vec![a0, a1],
            },
        ],
        capacity: vec![],
        obeying: vec![],
        demands: vec![],
        bounds: vec![AttackerBound {
            attacker: a0,
            min_blockers: 2,
            max_blockers: u32::MAX,
        }],
    };
    let cands = EncodedCandidates4 {
        head: HeadKind4::PairPick,
        pending_kind: crate::features4::PendingKind4::DeclareBlockers,
        min_choices: 0,
        max_choices: 2,
        candidates: Vec::new(),
        pairs: vec![[0, 2], [0, 3], [1, 2], [1, 3]],
        pair_objects: vec![(b0, a0), (b0, a1), (b1, a0), (b1, a1)],
    };
    (pending, cands)
}

/// A blocker draw the attackers' counts moved carries no behaviour
/// probability (`sample_pairs_constrained`). Where the explorer executes
/// that answer it stays without one: no π on the row and no μ in the record,
/// never zero and never a number's point mass. A layer answer played in its
/// place is drawn as it stands and records its own.
#[test]
fn a_blocker_draw_the_counts_moved_records_no_behaviour_probability() {
    let view = view();
    let (pending, cands) = menace_block(&view);
    // `b0` is drawn alone on the menace attacker, and `b1` joins it.
    let logits = [20.0_f32, -20.0, -20.0, -20.0];
    let (alone, lp, _) = decide(None, 1, &view, &pending, &cands, &logits, 0.2);
    assert_eq!(alone.multi_mask, [1, 0, 1, 0]);
    assert_eq!(lp, None, "the moved draw has no probability");
    let table = proposers();
    let mut explorer = explorer();
    let mut executed_net = 0;
    for seed in 0..64 {
        explorer.reseed(seed);
        let (target, lp, _) = decide(
            Some(&mut explorer),
            seed,
            &view,
            &pending,
            &cands,
            &logits,
            0.2,
        );
        let record = explorer.take_last().expect("an explored decision");
        let provenance = PolicyMode4::Raw.provenance(TAU, 2, HeadKind4::PairPick, lp);
        check_step(
            &record,
            EPSILON,
            provenance.has_logprob,
            provenance.behavior_logprob,
            &table,
        )
        .unwrap();
        if record.played {
            assert!(lp.is_some() && record.mixture_logprob.is_some());
        } else {
            executed_net += 1;
            assert_eq!(target.multi_mask, alone.multi_mask);
            assert_eq!((lp, record.mixture_logprob), (None, None), "{record:?}");
            assert!(!provenance.has_logprob);
            assert_eq!(provenance.behavior_logprob.to_bits(), 0);
        }
    }
    assert!(executed_net > 0, "the net's own answer is executed");
}

/// A game's coins are its seed's stream, one per explored decision.
#[test]
fn the_coins_are_the_seeded_stream() {
    let view = view();
    let pending = Pending::Mulligan {
        player: view.seat,
        taken: 0,
        next_is_free: false,
        can_take: true,
    };
    let cands = menu(&view, &pending);
    let mut explorer = explorer();
    explorer.reseed(0x29);
    let mut coins = Vec::new();
    for draw in 0..16 {
        decide(
            Some(&mut explorer),
            draw,
            &view,
            &pending,
            &cands,
            &[1.5, -0.5],
            0.2,
        );
        let record = explorer.take_last().unwrap();
        assert_eq!(record.draw, u32::try_from(draw).unwrap());
        coins.push(record.coin);
    }
    let game = explorer.game();
    assert_eq!(game.seed, 0x29 ^ exploration::SEED_SALT);
    assert_eq!(game.draws, 16);
    let mut stream = Rng::new(game.seed);
    assert!(
        coins
            .iter()
            .all(|&coin| coin.to_bits() == stream.unit().to_bits())
    );
}
