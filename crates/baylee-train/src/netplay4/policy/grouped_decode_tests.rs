//! `--grouped-decode` on a real priority menu: off it touches nothing, on it
//! moves a top pass to the best act when the acts together outweigh it, and
//! only at priority.
#![allow(
    clippy::cast_precision_loss,
    clippy::naive_bytecount,
    reason = "a test menu holds a handful of candidates"
)]
use super::*;
use crate::coarse_class::{PASS_CLASS, coarse_classes_of};
use baylee_core::preset::AIProfile;

/// A priority question of a real game with a pass and two or more other
/// answers, as a player without graphs meets it.
fn priority_menu() -> (NetPlayer4, PlayerView, Pending, EncodedCandidates4) {
    let deck = crate::housedeck::HouseDeck::named("victory").unwrap();
    let preset = crate::selfplay::table_for(42, &[&deck, &deck], &[AIProfile::STEADY; 2], &[]);
    let mut session = baylee_gamehost::Session::new(&preset).unwrap();
    for seat in 0..2 {
        session.take_over(PlayerId::new(seat));
    }
    let mut net = NetPlayer4::without_graphs(7, 16);
    net.init_game(&preset);
    let teams = [None, None];
    for _ in 0..400 {
        let player = session.pending().asked().expect("the game goes on");
        let (pending, view) = session.view_for_with_context(player).unwrap();
        let (_, cands) = net.policy_encoding(&view, &pending, &teams);
        let classes = coarse_classes_of(&cands, &view);
        let acts = classes.iter().filter(|&&class| class != PASS_CLASS).count();
        if matches!(pending, Pending::Priority { .. })
            && cands.head == HeadKind4::SingleChoice
            && acts >= 2
            && acts < classes.len()
        {
            return (net, view, pending, cands);
        }
        // Without graphs the player answers by its fallback, which is legal.
        let action = net.act(&view, &pending, &teams);
        session.act(player, action).unwrap();
    }
    panic!("no priority question with a pass and two other answers");
}

/// Pass on top at 40 %, the other answers sharing 60 % with none above 40 %.
fn split_logits(classes: &[u8]) -> Vec<f32> {
    let acts = classes.iter().filter(|&&class| class != PASS_CLASS).count();
    let passes = classes.len() - acts;
    let mut seen = 0_usize;
    classes
        .iter()
        .map(|&class| {
            if class == PASS_CLASS {
                (0.4 / passes as f32).ln()
            } else {
                seen += 1;
                // The first act a little ahead of the others, so the best is known.
                let share = 0.6 / acts as f32;
                (if seen == 1 {
                    share * 1.05
                } else {
                    share * (1.0 - 0.05 / (acts - 1) as f32)
                })
                .ln()
            }
        })
        .collect()
}

fn argmax(logits: &[f32]) -> usize {
    (0..logits.len())
        .max_by(|&a, &b| logits[a].total_cmp(&logits[b]))
        .unwrap()
}

#[test]
fn off_by_default_the_logits_are_bit_identical_and_nothing_is_counted() {
    let (mut net, view, pending, cands) = priority_menu();
    assert!(!net.grouped_decode());
    let classes = coarse_classes_of(&cands, &view);
    let before = split_logits(&classes);
    let mut logits = before.clone();
    assert_eq!(
        net.group_priority(&cands, &view, &pending, &mut logits),
        None
    );
    assert!(
        logits
            .iter()
            .zip(&before)
            .all(|(a, b)| a.to_bits() == b.to_bits())
    );
    assert_eq!(net.take_grouped_use(), GroupedUse4::default());
    assert_eq!(net.last_grouping(), None);
}

#[test]
fn a_top_pass_gives_way_to_the_best_act_when_the_acts_outweigh_it() {
    let (mut net, view, pending, cands) = priority_menu();
    net.set_grouped_decode(true);
    let classes = coarse_classes_of(&cands, &view);
    let passes = classes.iter().filter(|&&class| class == PASS_CLASS).count();
    let mut logits = split_logits(&classes);
    if passes == 1 {
        assert_eq!(
            classes[argmax(&logits)],
            PASS_CLASS,
            "pass is the plain top answer"
        );
    }
    let first_act = classes
        .iter()
        .position(|&class| class != PASS_CLASS)
        .unwrap();
    let grouping = net
        .group_priority(&cands, &view, &pending, &mut logits)
        .expect("both groups are on the menu");
    assert!(grouping.act_wins);
    assert!((grouping.act_mass - 0.6).abs() < 1e-4 && (grouping.pass_mass - 0.4).abs() < 1e-4);
    // The answer is the best candidate inside the winning group.
    assert_eq!(argmax(&logits), first_act);
    let used = net.take_grouped_use();
    assert_eq!(used.decisions, 1);
    assert_eq!(used.pass_to_act, u64::from(grouping.changed));
    assert_eq!(used.act_to_pass, 0);
    if passes == 1 {
        assert!(grouping.changed && grouping.decides(0.0));
    }

    // The sampler then answers from what is left: raw and greedy, the best act.
    let mut rng = Rng::new(5);
    let (action, _, _, target) = PolicySampler4 {
        tactician: None,
        mode: PolicyMode4::Raw,
        temperature: 0.0,
        rng: &mut rng,
        active_source: None,
        guard: &mut crate::netplay4::ChoiceGuard4::default(),
        rules: &mut crate::netplay4::TacticRules4::default(),
    }
    .select(
        &cands,
        &view,
        &pending,
        PolicyOutputs4 {
            embeddings: None,
            cand_logits: &logits,
            pair_logits: &[],
            scalar_pred: 0.0,
            value: 0.5,
        },
    )
    .unwrap();
    assert_eq!(usize::from(target.single_idx), first_act);
    assert!(!matches!(action, PlayerAction::PassPriority));
}

#[test]
fn a_question_that_is_not_priority_is_left_alone() {
    let (mut net, view, pending, cands) = priority_menu();
    net.set_grouped_decode(true);
    let classes = coarse_classes_of(&cands, &view);
    let before = split_logits(&classes);
    // The same menu under another head is no priority single choice.
    let mut other = cands.clone();
    other.head = HeadKind4::SubsetPick;
    let mut logits = before.clone();
    assert_eq!(
        net.group_priority(&other, &view, &pending, &mut logits),
        None
    );
    assert_eq!(logits, before);
    // Nor is a question that is not priority.
    let mulligan = Pending::Mulligan {
        player: view.seat,
        taken: 0,
        next_is_free: false,
        can_take: true,
    };
    let mut logits = before.clone();
    assert_eq!(
        net.group_priority(&cands, &view, &mulligan, &mut logits),
        None
    );
    assert_eq!(logits, before);
    assert_eq!(net.take_grouped_use(), GroupedUse4::default());
}
