//! The tactician's behavior, choice and rows on small cases.
#![allow(clippy::cast_precision_loss, reason = "small test indices")]
use super::head::assemble;
use super::*;

fn masks() -> (Vec<u8>, Vec<u8>, Vec<Vec<u8>>) {
    let net = vec![1, 0, 0];
    let rules = vec![0, 1, 0];
    let neighbors = vec![vec![0, 1, 0], vec![0, 0, 1], vec![1, 1, 0]];
    (net, rules, neighbors)
}

/// The probability of every answer the behavior can play, summed.
fn total(epsilon: f64, net: &[u8], rules: &[u8], neighbors: &[Vec<u8>]) -> f64 {
    let mut answers: Vec<Vec<u8>> = vec![net.to_vec(), rules.to_vec()];
    for n in neighbors {
        if !answers.contains(n) {
            answers.push(n.clone());
        }
    }
    answers.dedup();
    let mut seen = Vec::new();
    let mut sum = 0.0;
    // Find each answer's probability by drawing on a fine grid.
    for step in 0..100_000 {
        let draw = (f64::from(step) + 0.5) / 100_000.0;
        let (played, p, _) = explore(epsilon, draw, net, rules, neighbors);
        if !seen.contains(&played) {
            seen.push(played);
            sum += p;
        }
    }
    sum
}

#[test]
fn the_behavior_s_probabilities_are_exact_and_sum_to_one() {
    let (net, rules, neighbors) = masks();
    let eps = DEFAULT_EPSILON;
    let (_, p_net, drawn) = explore(eps, 0.0, &net, &rules, &neighbors);
    assert_eq!(drawn, Drawn4::Net);
    assert!((p_net - (1.0 - eps)).abs() < 1e-12);
    // The rules' answer is also a neighbor here: its probability is both.
    let (played, p_rules, drawn) = explore(eps, 1.0 - eps + 1e-9, &net, &rules, &neighbors);
    assert_eq!(
        (played.as_slice(), drawn),
        (rules.as_slice(), Drawn4::Rules)
    );
    let each = (eps - eps / 3.0) / 3.0;
    assert!((p_rules - (eps / 3.0 + each)).abs() < 1e-12);
    let (played, p, drawn) = explore(eps, 0.999_999, &net, &rules, &neighbors);
    assert_eq!((played, drawn), (vec![1, 1, 0], Drawn4::Neighbor));
    assert!((p - each).abs() < 1e-12);
    assert!((total(eps, &net, &rules, &neighbors) - 1.0).abs() < 1e-9);
    // Rules agreeing with the net: all of ε goes to the neighbors.
    assert!((total(eps, &net, &net, &neighbors) - 1.0).abs() < 1e-9);
    let (_, p, _) = explore(eps, 0.999_999, &net, &net, &neighbors);
    assert!((p - eps / 3.0).abs() < 1e-12);
    // Nothing to explore: the net with what the neighbors would have had.
    let (played, p, _) = explore(eps, 0.999_999, &net, &rules, &[]);
    assert_eq!(played, rules);
    assert!((p - eps / 3.0).abs() < 1e-12);
    let (played, p, _) = explore(eps, 0.5, &net, &rules, &[]);
    assert_eq!(played, net);
    assert!((p - (1.0 - eps / 3.0)).abs() < 1e-12);
}

#[test]
fn zero_exploration_plays_exactly_the_net() {
    let (net, rules, neighbors) = masks();
    for step in 0..1000 {
        let draw = f64::from(step) / 1000.0;
        let (played, p, drawn) = explore(0.0, draw, &net, &rules, &neighbors);
        assert_eq!(
            (played.as_slice(), p, drawn),
            (net.as_slice(), 1.0, Drawn4::Net)
        );
    }
}

#[test]
fn the_head_leaves_the_net_only_past_the_margin() {
    let (net, rules, neighbors) = masks();
    // Null, then units 0..3: unit 1 is worth 0.3 more than unit 0.
    let scores = [0.1, 0.0, 0.3, -0.2];
    assert_eq!(
        best_supported(&scores, &net, &rules, &neighbors, 0.0),
        vec![0, 1, 0]
    );
    assert_eq!(best_supported(&scores, &net, &rules, &neighbors, 0.3), net);
    // Adding unit 1 to the net's answer is the largest gain.
    let scores = [0.1, 0.2, 0.3, -0.2];
    assert_eq!(
        best_supported(&scores, &net, &rules, &neighbors, 0.0),
        vec![1, 1, 0]
    );
    // Ties keep the net.
    let flat = [0.0; 4];
    assert_eq!(best_supported(&flat, &net, &rules, &neighbors, 0.0), net);
}

#[test]
fn windows_parse_by_name() {
    let all = Window4::parse_list("all").unwrap();
    assert_eq!(all.len(), 3);
    assert_eq!(
        Window4::parse_list("block,target").unwrap(),
        BTreeSet::from([Window4::Block, Window4::Target])
    );
    assert!(Window4::parse_list("mulligan").is_err());
    assert!(Window4::parse_list("").is_err());
}

#[test]
fn rows_put_state_unit_and_tail_in_place_with_the_null_row_first() {
    let d = 2;
    let state = [1.0, 2.0];
    let units = [3.0, 4.0, 5.0, 6.0];
    let tails: Vec<f32> = (0..3 * ROW_TAIL).map(|i| i as f32).collect();
    let rows = assemble(&state, &units, &tails, 2);
    let width = 2 * d + ROW_TAIL;
    assert_eq!(rows.len(), 3 * width);
    assert_eq!(&rows[..4], &[1.0, 2.0, 0.0, 0.0]);
    assert_eq!(&rows[width..width + 4], &[1.0, 2.0, 3.0, 4.0]);
    assert_eq!(&rows[2 * width..2 * width + 4], &[1.0, 2.0, 5.0, 6.0]);
    assert_eq!(rows[2 * width + 4], tails[2 * ROW_TAIL]);
}

mod on_a_table {
    use super::super::look::LookPrior4;
    use super::super::*;
    use crate::netplay4::NetPlayer4;
    use crate::netplay4::search::{ForkSource4, KnownDecks};
    use baylee_core::ids::PlayerId;
    use baylee_core::preset::AIProfile;
    use baylee_gamehost::Session;

    const ME: PlayerId = PlayerId::new(0);
    const D: usize = 4;

    fn table(seed: u64) -> Session {
        let deck = crate::housedeck::HouseDeck::named("victory").unwrap();
        let preset =
            crate::selfplay::table_for(seed, &[&deck, &deck], &[AIProfile::STEADY; 2], &[]);
        let mut session = Session::new_recorded(&preset, "test").unwrap();
        session.describe("tactician".into(), vec!["0".into(), "1".into()]);
        assert!(session.take_over(ME));
        session
    }

    /// Drives the table, seat 0 by the steady house, until `want` holds of
    /// seat 0's question.
    fn drive(
        session: &mut Session,
        want: impl Fn(&PlayerView, &Pending) -> bool,
    ) -> Option<(Pending, PlayerView)> {
        for _ in 0..8000 {
            if matches!(session.pending(), Pending::GameOver(_)) {
                return None;
            }
            if session.needs_pump() {
                session.pump_at_most(1);
                continue;
            }
            let (pending, view) = session.view_for_with_context(ME)?;
            if want(&view, &pending) {
                return Some((pending, view));
            }
            let action = session.house_action(ME)?;
            session.act_slice(ME, action, 0).ok()?;
        }
        None
    }

    fn prior() -> Arc<LookPrior4> {
        let deck = crate::housedeck::HouseDeck::named("victory").unwrap();
        Arc::new(LookPrior4::from_decks([&deck.deck]).unwrap())
    }

    /// Plays the window `want` names on some table: the answer decodes, the
    /// record is whole, and every neighbor is a legal answer one change away.
    fn exercise(window: Window4, want: impl Fn(&PlayerView, &Pending) -> bool + Copy) {
        let net = NetPlayer4::without_graphs(10, D);
        let mut tactician = Tactician4::new(
            BTreeSet::from([window]),
            TacticianMode4::Explore { epsilon: 0.9 },
            prior(),
        )
        .unwrap();
        let mut met = 0;
        for seed in 1..40_u64 {
            let mut session = table(seed);
            tactician.reseed(seed);
            while let Some((pending, view)) = drive(&mut session, want) {
                let (_, cands) = net.policy_encoding(&view, &pending, &[None, None]);
                let units = window.units(&cands);
                let logits: Vec<f32> = (0..units).map(|i| (i as f32 * 0.7).sin()).collect();
                let embeddings = StepEmbeddings4 {
                    state: vec![0.5; D],
                    candidates: vec![0.25; cands.candidates.len() * D],
                    pairs: vec![0.125; cands.pairs.len() * D],
                };
                let rng = Rng::new(seed);
                let step = TacticianStep {
                    cands: &cands,
                    view: &view,
                    pending: &pending,
                    cand_logits: &logits,
                    pair_logits: &logits,
                    value: 0.5,
                    embeddings: Some(&embeddings),
                    active_source: None,
                    rng: &rng,
                };
                let target = tactician.answer(&step).unwrap().expect("a window answer");
                let action = decode_action4(&cands, &view, &pending, &target).unwrap();
                assert_eq!(pending.answer_fault(&action), None);
                let record = tactician.take_records().pop().unwrap();
                assert_eq!(record.window, window);
                assert_eq!(record.keys.len(), units);
                assert_eq!(record.rows.len(), (units + 1) * ROW_TAIL);
                assert_eq!(record.units.len(), units * D);
                assert!(record.probability > 0.0 && record.probability <= 1.0);
                for n in neighbors(window, &step, &record.net) {
                    assert_ne!(n, record.net);
                    assert!(exact_action(window, &step, &n).is_some());
                    let changed = n.iter().zip(&record.net).filter(|(a, b)| a != b).count();
                    assert!(changed <= 2, "{window:?}: {changed} units changed");
                }
                met += 1;
                session.act_slice(ME, action, 0).unwrap();
                if met >= 6 {
                    return;
                }
            }
        }
        assert!(met > 0, "no {window:?} question met");
    }

    #[test]
    fn attacks_are_explored_into_legal_answers() {
        exercise(
            Window4::Attack,
            |_, p| matches!(p, Pending::ChooseAttackers { attackers, .. } if attackers.len() >= 2),
        );
    }

    #[test]
    fn blocks_are_explored_into_legal_answers() {
        exercise(
            Window4::Block,
            |_, p| matches!(p, Pending::ChooseBlockers { blockers, .. } if !blockers.is_empty()),
        );
    }

    #[test]
    fn targets_are_explored_into_legal_answers() {
        exercise(Window4::Target, |_, p| {
            matches!(p, Pending::ChooseTargets { options, player_options, max: 1, .. }
                if options.len() + player_options.len() >= 2)
        });
    }

    /// Hidden cards reach the look only as the prior: two worlds dealt again
    /// from the same public position, the other side's hand and library
    /// different, give the same belief and the same view-read columns.
    #[test]
    fn the_other_sides_hidden_cards_never_reach_the_look() {
        let prior = prior();
        let mut session = table(7);
        let (_, view) = drive(&mut session, |v, p| {
            v.turn >= 5
                && matches!(p, Pending::Priority { .. })
                && v.seat(PlayerId::new(1)).is_some_and(|s| s.hand_count >= 2)
        })
        .expect("a priority decision on turn 5");
        if KnownDecks(&session).fork(ME, 1).is_err() {
            return;
        }
        let real = prior.belief_at(&view);
        let hands = |seed: u64| {
            let fork = KnownDecks(&session).fork(ME, seed).unwrap();
            let them = PlayerId::new(1);
            let hand: Vec<String> = fork
                .state()
                .zones
                .list(baylee_engine::zone::ZoneLocation::Hand(them))
                .iter()
                .filter_map(|&id| fork.state().arena.get(id).map(|o| format!("{:?}", o.card)))
                .collect();
            let (_, seen) = fork.view_for_with_context(ME).unwrap();
            (hand, prior.belief_at(&seen), seen)
        };
        let (hand_a, look_a, view_a) = hands(11);
        let differing = (12..200_u64).find_map(|seed| {
            let (hand, look, view) = hands(seed);
            (hand != hand_a).then_some((look, view))
        });
        let (look_b, view_b) = differing.expect("two worlds with different hidden hands");
        assert_eq!(look_a, real);
        assert_eq!(look_b, real);
        assert_eq!(
            view_a.seat(PlayerId::new(1)).map(|s| s.hand_count),
            view_b.seat(PlayerId::new(1)).map(|s| s.hand_count)
        );
    }
}

/// The belief is the prior's share of castable instant-speed cards of each
/// family, times the hand: nothing else.
#[test]
fn the_prior_expects_answers_by_share_colour_and_mana() {
    use baylee_core::color::{Color, ColorSet};
    let index = |name: &str| {
        baylee_cards::all()
            .find(|c| c.name() == name)
            .unwrap_or_else(|| panic!("{name} is a card"))
            .index
            .get()
    };
    let prior = look::LookPrior4::of_cards(&[
        (index("Counterspell"), 4.0),
        (index("Lightning Bolt"), 4.0),
        (index("Island"), 12.0),
    ])
    .unwrap();
    let counter = crate::cardstructure::labels::FAMILIES
        .iter()
        .position(|f| *f == "counter_spell")
        .unwrap();
    let damage = 0;
    let blue = ColorSet::of(Color::Blue);
    // Blue seen: Lightning Bolt is not theirs to hold; 4 of 16 cards counter.
    let (by_family, all) = prior.expected_answers(blue, 4, 2, blue);
    assert!((by_family[counter] - 1.0).abs() < 1e-6);
    assert_eq!(by_family[damage], 0.0);
    assert!((all - 1.0).abs() < 1e-6);
    // One open mana pays for no Counterspell.
    let (by_family, _) = prior.expected_answers(blue, 4, 1, blue);
    assert_eq!(by_family[counter], 0.0);
    // Nothing seen yet: every card is theirs to hold, Bolt among them.
    let red = ColorSet::of(Color::Red);
    let (by_family, _) = prior.expected_answers(ColorSet::EMPTY, 5, 1, red);
    assert!((by_family[damage] - 1.0).abs() < 1e-6);
    assert_eq!(by_family[counter], 0.0);
}
